//! Simple File System
use crate::DEVICE;
use crate::drivers::Storage;
use crate::error::Error;
use crate::error::Result;
use alloc::vec;
use core::cmp::max;
use core::cmp::min;
use core::fmt::Debug;
use core::num::NonZeroUsize;
use core::{array, slice};
use core::{cell::Cell, ptr};

use alloc::rc::Rc;

use alloc::vec::Vec;

use crate::fs::caching::{self, Interval, Lru};

pub const BLOCK_SIZE: usize = 4096; // in bytes
pub const SIZE: usize = 104857600;

pub const INODES: usize = 200;
pub const BITMAP_BITS_PER_BLOCK: usize = BLOCK_SIZE * 8; // in bits
pub const BITMAP_BLOCKS: usize = 100;
pub const INODE_SIZE: usize = 128;
pub const INODE_PER_BLOCK: usize = BLOCK_SIZE / INODE_SIZE;
pub const INODE_BLOCKS: usize = 25;
pub const TOTAL_BLOCKS: usize = SIZE / BLOCK_SIZE;
pub const DATA_BLOCKS: usize = TOTAL_BLOCKS - (1 + BITMAP_BLOCKS + INODE_BLOCKS);
pub const BITMAP_START: usize = 1;
pub const INODE_START: usize = BITMAP_BLOCKS + BITMAP_START;
pub const DATA_START: usize = INODE_START + INODE_BLOCKS;

pub const DIRECT: usize = 12;
pub const MAGIC: usize = 0x238437378298386;
pub const DISK_INODE_SIZE: usize = size_of::<DiskINode>();
pub const MEMORY_INODE_SIZE: usize = size_of::<MemoryINode>();
pub const INDIRECT: usize = 128;
pub const LRU_CACHE_CAPACITY: usize = 512;

pub static mut INODE_CACHE: Vec<Rc<MemoryINode>> = Vec::new();

pub static mut DATA_CACHE: Lru<usize, Interval> = Lru::new();

pub static mut UNWRITTEN_INODES: usize = 0;
pub static mut UNWRITTEN_DATA_BLOCKS: usize = 0;

pub const MAXIMUM_FILE_SIZE: usize = (DIRECT + INDIRECT) * BLOCK_SIZE;
pub const FILE_NAME_SIZE: usize = 248;

#[repr(C)]
#[derive(Debug)]
pub struct SuperBlock {
    pub magic: usize,
    pub data_blocks: usize,
    pub inode_blocks: usize,
    pub bitmap_blocks: usize,
    pub total_blocks: usize,
    pub bitmap_start: usize,
    pub inode_start: usize,
    pub data_start: usize,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InodeEntry {
    File,
    Directory,
    Device,
    SymLink,
    None,
}

#[repr(C)]
#[derive(Debug)]
pub struct DirectoryEntry {
    pub name: [u8; FILE_NAME_SIZE],
    pub inum: usize,
}

#[repr(C)]
pub struct MemoryINode {
    pub entry: Cell<InodeEntry>,
    pub major: Cell<u8>,
    pub size: Cell<usize>,
    pub links: Cell<usize>,
    pub inum: NonZeroUsize,
    pub needs_write: Cell<bool>,
    pub data: [Cell<usize>; DIRECT + 1],
    pub device: &'static dyn Storage,
}

impl Default for MemoryINode {
    fn default() -> Self {
        Self {
            entry: Cell::new(InodeEntry::None),
            major: Cell::new(0),
            size: Cell::new(0),
            links: Cell::new(0),
            inum: NonZeroUsize::new(1).unwrap(),
            needs_write: Cell::new(false),
            data: array::from_fn(|_| Cell::new(0)),
            device: &DEVICE,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DiskINode {
    entry: InodeEntry,
    major: u8,
    size: usize,
    links: usize,
    data: [usize; DIRECT + 1],
}

pub struct DataBlock {
    data: Cell<[u8; BLOCK_SIZE]>,
    needs_write: Cell<bool>,
}

impl From<&MemoryINode> for DiskINode {
    fn from(inode: &MemoryINode) -> Self {
        let data = array::from_fn(|i| inode.data[i].get());

        Self {
            entry: inode.entry.get(),
            major: inode.major.get(),
            size: inode.size.get(),
            links: inode.links.get(),
            data,
        }
    }
}

impl Drop for MemoryINode {
    fn drop(&mut self) {
        let device = self.device;

        if self.links.get() == 0 {
            let disk_inode = DiskINode::from(&*self);

            let (block, offset) = calculate_block_from_inum(self.inum, device);

            let mut buffer = device.read_blocks(block, 1);

            let dest = unsafe {
                &mut *(buffer[offset..offset + size_of::<DiskINode>()].as_mut_ptr()
                    as *mut DiskINode)
            };

            *dest = disk_inode;

            device.write_blocks(block, 1, &buffer);
        }
    }
}

/// Searches the bitmap blocks for a free block and returns an i-number.
pub fn allocate_inode(device: &dyn Storage) -> Result<NonZeroUsize> {
    let super_block = device.read_blocks(0, 1);
    let super_block = unsafe { &*(super_block.as_ptr() as *const SuperBlock) };

    let bitmap_bytes = (INODES + 7) / 8;

    // use integer ceiling formula `ceil(x / y) = (x + y - 1) / y`
    let bitmap_blocks = (bitmap_bytes + BLOCK_SIZE - 1) / BLOCK_SIZE;

    let mut bitmaps = device.read_blocks(super_block.bitmap_start, bitmap_blocks);

    for i in 0..bitmaps.len() {
        if i >= bitmap_bytes {
            break;
        }

        if bitmaps[i] == 0xFF {
            continue;
        }

        for j in 0..8 {
            let inum = i * 8 + j + 1;

            if inum > INODES {
                return Err(Error::NoFreeINode);
            }

            let old = bitmaps[i];
            if old & (1 << j) == 0 {
                let new = old | (1 << j);
                bitmaps[i] = new;
                device.write_blocks(super_block.bitmap_start, bitmap_blocks, &bitmaps);

                return NonZeroUsize::new(inum).ok_or(Error::InumZero);
            }
        }
    }

    Err(Error::NoFreeINode)
}

/// Calculates and returns the location of inode on device. The first value of the tuple returned
///  is the block and the second value is the offset within the block.
pub fn calculate_block_from_inum(inum: NonZeroUsize, device: &dyn Storage) -> (usize, usize) {
    let inum = inum.get();
    assert!((1 <= inum) && (inum <= INODES));

    let super_block = device.read_blocks(0, 1);
    let super_block = unsafe { &*(super_block.as_ptr() as *const SuperBlock) };

    let iindex = inum - 1;

    let inode_offset_from_inode_start = iindex * DISK_INODE_SIZE;
    let inode_block_number_from_inode_start = inode_offset_from_inode_start / BLOCK_SIZE;

    let inode_offset_within_block = inode_offset_from_inode_start % BLOCK_SIZE;
    let inode_block_number = inode_block_number_from_inode_start + super_block.inode_start;

    (inode_block_number, inode_offset_within_block)
}

/// Reads and returns an inode corresponding to the passed i-number from the storage device
pub fn read_inode(inum: NonZeroUsize, device: &'static dyn Storage) -> Rc<MemoryINode> {
    // check if the inode is in the cache, if it is return it, else load from device
    // and add it to the cache

    for inode in unsafe { &INODE_CACHE } {
        if inode.inum == inum {
            return inode.clone();
        }
    }

    let (block, offset) = calculate_block_from_inum(inum, device);

    let block = device.read_blocks(block, 1);

    let disk_inode =
        unsafe { &*(block[offset..offset + size_of::<DiskINode>()].as_ptr() as *const DiskINode) }
            .clone();

    let inode = Rc::new(MemoryINode {
        entry: Cell::new(disk_inode.entry),
        major: Cell::new(disk_inode.major),
        size: Cell::new(disk_inode.size),
        links: Cell::new(disk_inode.links),
        inum: inum,
        data: array::from_fn(|i| Cell::new(disk_inode.data[i])),
        needs_write: Cell::new(false),
        device,
    });

    // inode not in cache, was read from device, so add it to the cache
    unsafe {
        INODE_CACHE.push(inode.clone());
    }

    return inode;
}

/// Takes a `MemoryInode` and writes a `DiskInode` to the device
pub fn write_inode(inode: Rc<MemoryINode>, device: &dyn Storage, force: bool) -> Result<()> {
    if inode.needs_write.get() == false {
        inode.needs_write.set(true);
        unsafe {
            UNWRITTEN_INODES += 1;
        }
    }

    let mut cached = false;

    for i in unsafe { &INODE_CACHE } {
        if i.inum == inode.inum {
            cached = true;
            break;
        }
    }

    if !cached {
        unsafe {
            INODE_CACHE.push(inode.clone());
        }
    }

    if force == true {
        flush_inodes(device)?;
    }

    if unsafe { UNWRITTEN_INODES } > 50 {
        flush_inodes(device)?;
    }

    Ok(())
}

pub fn flush_inodes(device: &dyn Storage) -> Result<()> {
    for inode in unsafe { &INODE_CACHE } {
        if inode.needs_write.get() {
            let disk_inode = DiskINode::from(inode.as_ref());

            let (block, offset) = calculate_block_from_inum(inode.inum, device);
            let mut buffer = device.read_blocks(block, 1);
            let dest = unsafe {
                &mut *(buffer[offset..offset + size_of::<DiskINode>()].as_mut_ptr()
                    as *mut DiskINode)
            };
            *dest = disk_inode;

            if let Some(key) = unsafe { DATA_CACHE.range(..=block).next_back() }
                && let Some(interval) = unsafe { DATA_CACHE.get_mut(key) }
                && interval.start <= block
                && interval.end > block
            {
                interval.needs_write = true;

                unsafe { UNWRITTEN_DATA_BLOCKS += interval.data.len() / BLOCK_SIZE }

                interval
            } else {
                let interval =
                    caching::coalesce(Interval::new(block, block + 1, buffer, true)?, None, None);

                unsafe { UNWRITTEN_DATA_BLOCKS += interval.data.len() / BLOCK_SIZE }

                unsafe {
                    DATA_CACHE.insert(interval.start, interval.clone());
                    DATA_CACHE.get_mut(&interval.start).unwrap()
                }
            };

            inode.needs_write.set(false);
        }
    }

    flush_data_blocks(device, true);

    Ok(())
}

/// Searches the file system for a block to store data and returns a zero-indexed block number
pub fn allocate_data_block(device: &dyn Storage) -> Result<usize> {
    let super_block = &device.read_blocks(0, 1)[..];
    let super_block = unsafe { &*(super_block.as_ptr() as *const SuperBlock) };

    let inode_bitmap_bytes = (INODES + 7) / 8;
    let number_of_inode_bitmap_blocks = (inode_bitmap_bytes + BLOCK_SIZE - 1) / BLOCK_SIZE;

    let number_of_data_bitmap_blocks = super_block.bitmap_blocks - number_of_inode_bitmap_blocks;

    let mut bitmaps = device.read_blocks(
        super_block.bitmap_start + number_of_inode_bitmap_blocks,
        number_of_data_bitmap_blocks,
    );

    for i in 0..bitmaps.len() {
        if bitmaps[i] == 0xFF {
            continue;
        }

        let old = bitmaps[i];
        for j in 0..8 {
            if old & (1 << j) == 0 {
                let new = old | (1 << j);
                bitmaps[i] = new;

                device.write_blocks(
                    super_block.bitmap_start + number_of_inode_bitmap_blocks,
                    number_of_data_bitmap_blocks,
                    &bitmaps,
                );

                return Ok(super_block.data_start + (i * 8 + j));
            }
        }
    }

    Err(Error::NoFreeDataBlock)
}

/// Converts a logical block in file to physical block number on the device.
///
/// For example, the third block of a file may be the 789th block on the device. This function converts
/// the three into 789.
pub fn logical_block_to_physical_block(
    inode: &Rc<MemoryINode>,
    mut logical_block: usize,
    device: &dyn Storage,
    allocate: bool,
) -> Result<usize> {
    if logical_block < DIRECT {
        match inode.data[logical_block].get() {
            0 => {
                if allocate {
                    let block = allocate_data_block(device)?;
                    inode.data[logical_block].set(block);
                    inode.needs_write.set(true);
                    flush_inodes(device)?;
                    Ok(block)
                } else {
                    Err(Error::NoBlockOnDevice)
                }
            }

            v => Ok(v),
        }
    } else {
        logical_block = logical_block - DIRECT;

        assert!(logical_block < INDIRECT);

        let indirect = inode.data[DIRECT].get();
        if indirect == 0 {
            if allocate {
                let block_containing_pointers_address = allocate_data_block(device)?;
                inode.data[DIRECT].set(block_containing_pointers_address);
                inode.needs_write.set(true);

                let mut block_containing_pointers = vec![0; BLOCK_SIZE];

                let data_block = allocate_data_block(device)?;

                unsafe {
                    ptr::copy_nonoverlapping(
                        u64::to_le_bytes(data_block as u64).as_ptr(),
                        block_containing_pointers[logical_block * 8..logical_block * 8 + 8]
                            .as_mut_ptr(),
                        8,
                    );
                }

                let interval = caching::coalesce(
                    Interval::new(
                        block_containing_pointers_address,
                        block_containing_pointers_address + 1,
                        block_containing_pointers,
                        true,
                    )?,
                    None,
                    None,
                );

                unsafe {
                    DATA_CACHE.insert(block_containing_pointers_address, interval);
                }

                flush_data_blocks(device, false);

                return Ok(data_block);
            }
            return Err(Error::NoBlockOnDevice);
        }

        let mut blocks = device.read_blocks(indirect, 1);

        let mut buffer = [0u8; 8];

        buffer.copy_from_slice(&blocks[logical_block * 8..logical_block * 8 + 8]);

        let ptr = u64::from_le_bytes(buffer) as usize;

        if ptr != 0 {
            return Ok(ptr);
        } else {
            if allocate {
                let new_block = allocate_data_block(device)?;
                blocks[logical_block * 8..logical_block * 8 + 8]
                    .copy_from_slice(&(new_block as u64).to_le_bytes());

                let interval = caching::coalesce(
                    Interval::new(indirect, indirect + 1, blocks, true)?,
                    None,
                    None,
                );

                unsafe {
                    DATA_CACHE.insert(indirect, interval);
                }

                flush_data_blocks(device, false);

                return Ok(new_block);
            }
        }
        Err(Error::NoBlockOnDevice)
    }
}

pub fn read_inode_data(
    inode: &Rc<MemoryINode>,
    byte_offset: usize,
    num_bytes: usize,
    read_beyond_eof_intended: bool,
    device: &dyn Storage,
) -> Result<Vec<u8>> {
    if byte_offset >= inode.size.get() {
        if !read_beyond_eof_intended {
            return Err(Error::ReadBeyondEOF);
        }

        // `lseek` accepts offsets greater than file size,
        // in such cases subsequent reads must return 0 bytes
        write_inode_data(
            inode,
            inode.size.get(),
            vec![0u8; byte_offset - inode.size.get()],
            device,
        )?;
    }

    let mut logical_block = byte_offset / BLOCK_SIZE;
    let mut offset_in_logical_block = byte_offset % BLOCK_SIZE;
    let mut read = 0;
    let mut data = Vec::new();

    while read < num_bytes {
        let mut offset_from_start: usize;
        let count = min(num_bytes - read, BLOCK_SIZE - offset_in_logical_block);

        let physical_block = logical_block_to_physical_block(inode, logical_block, device, false)?;

        let interval = if let Some(key) = unsafe { DATA_CACHE.range(..=physical_block).next_back() }
            && let Some(interval) = unsafe { DATA_CACHE.get(key) }
            && interval.start <= physical_block
            && interval.end > physical_block
        {
            offset_from_start = (physical_block - interval.start) * BLOCK_SIZE;

            interval
        } else {
            let block = device.read_blocks(physical_block, 1);
            offset_from_start = 0;

            let interval = caching::coalesce(
                Interval::new(physical_block, physical_block + 1, block, false)?,
                Some(&mut |predecessor_start| {
                    offset_from_start = (physical_block - predecessor_start) * BLOCK_SIZE
                }),
                None,
            );

            unsafe {
                let start = interval.start;
                DATA_CACHE.insert(start, interval);

                // this unwrap is safe because we just inserted the interval
                &DATA_CACHE.get(&start).unwrap()
            }
        };

        data.append(
            &mut interval.data[offset_from_start + offset_in_logical_block
                ..offset_from_start + offset_in_logical_block + count]
                .to_vec(),
        );

        read += count;
        offset_in_logical_block = 0;
        logical_block += 1;
    }

    Ok(data)
}

pub fn write_inode_data(
    inode: &Rc<MemoryINode>,
    byte_offset: usize,
    buffer: Vec<u8>,
    device: &dyn Storage,
) -> Result<()> {
    if inode.size.get() == 0 && byte_offset != 0 {
        return Err(Error::ByteOffsetNotZeroWhenInodeSizeIsZero);
    }

    let mut logical_block = byte_offset / BLOCK_SIZE;
    let mut offset_in_logical_block = byte_offset % BLOCK_SIZE;

    let num_bytes = if inode.size.get() != 0 {
        if buffer.len() > MAXIMUM_FILE_SIZE - byte_offset {
            return Err(Error::FileSizeOverflow);
        }
        // if writing the buffer does not cause the file size to increase
        // beyond the maximum file size,  write the entire buffer
        else {
            buffer.len()
        }
    } else {
        if buffer.len() > MAXIMUM_FILE_SIZE {
            return Err(Error::FileSizeOverflow);
        } else {
            buffer.len()
        }
    };

    let mut written = 0;

    while written < num_bytes {
        let physical_block = logical_block_to_physical_block(inode, logical_block, device, true)?;

        let count = min(num_bytes - written, BLOCK_SIZE - offset_in_logical_block);
        let mut offset_from_start: usize;

        let interval = if let Some(key) = unsafe { DATA_CACHE.range(..=physical_block).next_back() }
            && let Some(interval) = unsafe { DATA_CACHE.get_mut(key) }
            && interval.start <= physical_block
            && interval.end > physical_block
        {
            offset_from_start = (physical_block - interval.start) * BLOCK_SIZE;

            interval
        } else {
            let block = device.read_blocks(physical_block, 1);
            offset_from_start = 0;

            let interval = caching::coalesce(
                Interval::new(physical_block, physical_block + 1, block, false)?,
                Some(&mut |predecessor_start| {
                    offset_from_start = (physical_block - predecessor_start) * BLOCK_SIZE
                }),
                None,
            );

            unsafe {
                let start = interval.start;
                DATA_CACHE.insert(start, interval);

                // this unwrap is safe because we just inserted the interval
                DATA_CACHE.get_mut(&start).unwrap()
            }
        };

        interval.data[offset_from_start + offset_in_logical_block
            ..offset_from_start + offset_in_logical_block + count]
            .copy_from_slice(&buffer[written..written + count]);

        interval.needs_write = true;

        unsafe { UNWRITTEN_DATA_BLOCKS += interval.data.len() / BLOCK_SIZE }

        written += count;
        offset_in_logical_block = 0;
        logical_block += 1;
    }

    inode.size.set(max(byte_offset + written, inode.size.get()));
    flush_data_blocks(device, false);

    Ok(())
}

pub fn flush_data_blocks(device: &dyn Storage, force: bool) {
    if unsafe { UNWRITTEN_DATA_BLOCKS } > 50 || force {
        for (_, interval) in unsafe { &mut DATA_CACHE } {
            if interval.needs_write {
                device.write_blocks(
                    interval.start,
                    interval.end - interval.start,
                    &interval.data,
                );

                interval.needs_write = false;

                unsafe { UNWRITTEN_DATA_BLOCKS -= interval.end - interval.start }
            }
        }
    }
}

pub fn initialise(device: &'static dyn Storage) {
    let super_block = SuperBlock {
        magic: MAGIC,
        data_blocks: DATA_BLOCKS,
        inode_blocks: INODE_BLOCKS,
        bitmap_blocks: BITMAP_BLOCKS,
        total_blocks: TOTAL_BLOCKS,
        bitmap_start: BITMAP_START,
        inode_start: INODE_START,
        data_start: DATA_START,
    };

    device.write_blocks(0, 1, unsafe {
        core::slice::from_raw_parts(
            &raw const super_block as *const SuperBlock as *const u8,
            size_of::<SuperBlock>(),
        )
    });
}

pub fn free_inode(inode: &Rc<MemoryINode>) {}
