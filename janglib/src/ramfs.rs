use core::{
    mem,
    ptr::{NonNull, slice_from_raw_parts},
};

use alloc::vec::Vec;

const MAGIC: [u8; 4] = [0x93, 0x24, 0x72, 0x35];
const MAX_NAME_LENGTH: usize = 255;

#[derive(Debug, Clone, Copy)]
pub struct RamFsHeader {
    pub magic: [u8; 4],
    pub number_of_entries: [u8; 4],
}

#[derive(Debug, Clone, Copy)]
pub struct RamFsEntryHeader {
    pub magic: [u8; 4],
    pub name: [u8; MAX_NAME_LENGTH],
    pub size: [u8; 4],
}

impl RamFsHeader {
    pub fn new(number_of_entries: u32) -> Self {
        Self {
            magic: MAGIC,
            number_of_entries: number_of_entries.to_be_bytes(),
        }
    }

    pub fn number_of_files(&self) -> u32 {
        u32::from_be_bytes(self.number_of_entries)
    }

    pub fn set_number_of_files(&mut self, no: u32) {
        self.number_of_entries = no.to_be_bytes();
    }
}

impl RamFsEntryHeader {
    pub fn name(&self) -> &str {
        str::from_utf8(self.name.as_slice()).unwrap()
    }

    pub fn from_addr(start: usize) -> Self {
        let start = start as *const Self;
        assert!(
            unsafe { (*start).magic == MAGIC },
            "MAGIC is {:?}",
            unsafe { (*start).magic }
        );
        unsafe { *start }
    }

    pub fn size(&self) -> u32 {
        u32::from_be_bytes(self.size)
    }

    pub fn new(name: &str, size: u32) -> Self {
        let mut name_buf = [0u8; MAX_NAME_LENGTH];
        name_buf[0..MAX_NAME_LENGTH.min(name.len())]
            .copy_from_slice(&name.as_bytes()[0..MAX_NAME_LENGTH.min(name.len())]);
        Self {
            magic: MAGIC,
            name: name_buf,
            size: size.to_be_bytes(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RamFs {
    start: usize,
}

impl RamFs {
    pub fn number_of_files(&self) -> u32 {
        let start = unsafe { (self.start as *const RamFsHeader).as_ref().unwrap() };
        assert!(start.magic == MAGIC, "{:?}", start.magic);
        start.number_of_files()
    }

    pub fn set_number_of_files(&mut self, no: u32) {
        let start = unsafe { (self.start as *mut RamFsHeader).as_mut().unwrap() };
        assert!(start.magic == MAGIC, "{:?}", start.magic);
        start.set_number_of_files(no);
    }

    pub fn header(&self) -> [u8; size_of::<RamFsHeader>()] {
        let header = *unsafe { (self.start as *const RamFsHeader).as_ref().unwrap() };
        unsafe { mem::transmute::<_, [u8; size_of::<RamFsHeader>()]>(header) }
    }
}

pub struct RamFsIter {
    position: u32,
    ramfs: RamFs,
}

impl Iterator for RamFsIter {
    type Item = NonNull<RamFsEntryHeader>;

    fn next(&mut self) -> Option<Self::Item> {
        let data_start = unsafe { (self.ramfs.start as *const u8).add(8) };

        if self.position >= self.ramfs.number_of_files() {
            return None;
        }

        let mut i = 0;
        for _ in 0..=self.position {
            if i == self.position {
                return unsafe {
                    Some(NonNull::new(data_start.add(i as usize) as *mut RamFsEntryHeader).unwrap())
                };
            } else {
                i += unsafe {
                    u32::from_be_bytes(
                        (data_start.add(i as usize) as *mut RamFsEntryHeader)
                            .as_ref()
                            .unwrap()
                            .size,
                    )
                }
            }
        }
        None
    }
}

impl IntoIterator for RamFs {
    type Item = NonNull<RamFsEntryHeader>;

    type IntoIter = RamFsIter;

    fn into_iter(self) -> Self::IntoIter {
        RamFsIter {
            position: 0,
            ramfs: self.clone(),
        }
    }
}

impl RamFs {
    pub fn new() -> RamFsHeader {
        RamFsHeader {
            magic: MAGIC,
            number_of_entries: [0u8; 4],
        }
    }

    pub fn from_addr(start: usize) -> Self {
        let start = start as *const u8;
        assert!(unsafe { *(start as *const RamFsHeader) }.magic == MAGIC);
        let mut num_buf = [0u8; 4];
        let number_of_entries = unsafe { &*slice_from_raw_parts(start.add(4), 4) };
        num_buf.copy_from_slice(number_of_entries);
        let number_of_entries = u32::from_be_bytes(num_buf);
        {
            let mut start = unsafe { start.add(8) };
            assert!(!start.is_null());

            for _ in 0..number_of_entries {
                let h = RamFsEntryHeader::from_addr(start.addr());
                start = unsafe { start.add(u32::from_be_bytes(h.size) as usize) };
            }
        }
        RamFs {
            start: start.addr(),
        }
    }

    pub fn add_file(&mut self, name: &str, buf: &[u8]) -> Vec<u8> {
        let buf = buf.iter().map(|e| *e).collect::<Vec<u8>>();
        let header = RamFsEntryHeader::new(name, buf.len() as u32);
        let header = unsafe { mem::transmute::<_, [u8; size_of::<RamFsEntryHeader>()]>(header) };
        let mut header = header.into_iter().collect::<Vec<u8>>();
        header.extend(buf);
        self.set_number_of_files(self.number_of_files() + 1);
        header
    }
}
