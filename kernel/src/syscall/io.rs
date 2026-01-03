use core::{
    cmp::min,
    mem,
    num::{NonZero, NonZeroUsize},
    ops::{BitAnd, Index, Neg},
    ptr::slice_from_raw_parts_mut,
    str,
};

use alloc::{
    format, slice,
    string::String,
    vec::{self, Vec},
};
use ringbuffer::RingBuffer;
use riscv::register::satp::Mode;

use crate::{
    DEVICE,
    drivers::uart::{self, INPUT_BUFFER, READ, console_write, console_write_bytes},
    file::{self, FILES, File, FileType, STDERR, STDIN, STDOUT, allocate_file},
    fs::sfs::{
        FILE_NAME_SIZE, InodeEntry, flush_data_blocks, flush_inodes, free_inode, read_inode,
        read_inode_data, write_inode_data,
    },
    syscall::stdout,
    traps::TrapFrame,
    vm::translate_virtual_address,
};

#[repr(C)]
#[allow(non_camel_case_types)]
pub enum Flag {
    O_RDONLY = 0,
    O_WRONLY = 1,
    O_RDWR = 2,
    O_ACCMODE = 3,
    O_APPEND = 0x400,
    O_CREAT = 0x40,
    O_TRUNC = 0x200,
    O_EXCL = 0x80,
}

impl BitAnd<Flag> for usize {
    type Output = usize;

    fn bitand(self, rhs: Flag) -> Self::Output {
        self & rhs as usize
    }
}

#[repr(C)]
pub enum Error {
    EEXIST,

    /// `O_CREAT` is not set and file DNE or `O_CREAT` is set but path prefix DNE
    /// or path prefix is an empty string.
    ENOENT,
    ENOTDIR,
    EOVERFLOW,
    EACCES,
    ENAMETOOLONG,
    EBADF, // file descriptor is not associated with an open file
    EFBIG, // no room for bytes to be written
    ENXIO, // the operation is outside the capabilities of device
    ENOSPC,
    EINVAL,
    EPIPE,
}

impl Neg for Error {
    type Output = isize;

    fn neg(self) -> Self::Output {
        -(self as isize)
    }
}

pub fn open(trapframe: &TrapFrame) -> usize {
    let ptr = translate_virtual_address(trapframe.page_table, trapframe.a0)
        .unwrap_or_else(|e| panic!("OPEN FAILED - {}", e)) as *const u8;
    let flag = trapframe.a1;

    let mut path = Vec::new();
    let mut ch = unsafe { *ptr };

    let mut i = 0;
    while ch != '\0' as u8 {
        if (32..=126).contains(&ch) {
            path.push(ch);
            i += 1;
            ch = unsafe { *ptr.offset(i) };
        } else {
            // intentionally limiting to printable ASCII characters
            panic!("PROCESS TRIED TO OPEN PATH CONTAINING INVALID CHARACTERS");
        }
    }

    if let Ok(path) = str::from_utf8(&path) {
        if path.len() > FILE_NAME_SIZE {
            return -Error::ENAMETOOLONG as usize;
        }

        let file = allocate_file();
        let mode = flag & Flag::O_ACCMODE;

        let mut create = false;
        let mut truncate = false;
        let mut append = false;
        let mut excl = false;

        if mode == Flag::O_RDONLY as usize {
            *file.readable.borrow_mut() = true;
        } else if mode == Flag::O_WRONLY as usize {
            *file.writeable.borrow_mut() = true;
        } else if mode == Flag::O_RDWR as usize {
            *file.readable.borrow_mut() = true;
            *file.writeable.borrow_mut() = true;
        }

        if flag & Flag::O_CREAT as usize != 0 {
            create = true;
        }
        if flag & Flag::O_TRUNC as usize != 0 {
            if mode & Flag::O_RDONLY as usize != 0 {
                return -Error::EINVAL as usize;
            }
            truncate = true;
        }
        if flag & Flag::O_APPEND as usize != 0 {
            if mode & Flag::O_RDONLY as usize != 0 {
                return -Error::EINVAL as usize;
            }
            append = true;
        }
        if flag & Flag::O_EXCL as usize != 0 {
            excl = true;
        }

        match file::open(
            path,
            *file.readable.borrow(),
            *file.writeable.borrow(),
            create,
            excl,
            truncate,
            append,
        ) {
            Ok(fd) => fd,
            Err(e) => match e {
                crate::error::Error::NoSuchEntryInDirectory { name: _ }
                | crate::error::Error::FileDoesNotExist { path: _ }
                    if !create =>
                {
                    return -Error::ENOENT as usize;
                }

                crate::error::Error::FileAlreadyExists { path: _ } => {
                    return -Error::EEXIST as usize;
                }

                crate::error::Error::NotADirectory { name: _ } => return -Error::ENOTDIR as usize,

                crate::error::Error::NoFreeINode => return -Error::ENOSPC as usize,

                _ => panic!("{}", e),
            },
        }
    } else {
        panic!("PROCESS TRIED TO OPEN PATH CONTAINING INVALID CHARACTERS");
    }
}

pub fn write(trapframe: &TrapFrame) -> usize {
    let fd = trapframe.a0;
    let num_bytes = trapframe.a2;

    let buffer = unsafe {
        slice::from_raw_parts(
            translate_virtual_address(trapframe.page_table, trapframe.a1)
                .unwrap_or_else(|e| panic!("WRITE FAILED - {}", e)) as *mut u8,
            num_bytes,
        )
    };

    match unsafe { FILES.get(&fd) } {
        Some(file) => {
            if fd == STDIN {
                return -Error::EACCES as usize;
            } else if fd == STDOUT {
                console_write_bytes(&buffer);
                return num_bytes;
            } else if fd == STDERR {
                console_write_bytes(&buffer);
                return num_bytes;
            } else {
                if *file.writeable.borrow() == false {
                    return -Error::EACCES as usize;
                }

                match &*file.file_type.borrow() {
                    FileType::Pipe(pipe) => {
                        if let Err(e) = pipe.write(&buffer) {
                            if e == crate::error::Error::PipeReaderClosed
                                || e == crate::error::Error::PipeWriterClosed
                            {
                                return -Error::EPIPE as usize;
                            } else {
                                panic!("PIPE WRITE - {:?}", e);
                            }
                        }

                        buffer.len()
                    }

                    FileType::INode {
                        inode,
                        offset: _,
                        append,
                    } => {
                        if inode.entry.get() == InodeEntry::File {
                            let inode = read_inode(
                                unsafe { NonZeroUsize::new_unchecked(inode.inum.get()) },
                                &DEVICE,
                            );

                            if let Err(e) = write_inode_data(
                                &inode,
                                if *append { inode.size.get() } else { 0 },
                                buffer[..num_bytes].to_vec(),
                                &DEVICE,
                            ) {
                                match e {
                                    crate::error::Error::FileSizeOverflow => {
                                        return -Error::EOVERFLOW as usize;
                                    }
                                    crate::error::Error::NoFreeDataBlock => {
                                        return -Error::EFBIG as usize;
                                    }
                                    other => panic!("ERROR DURING SYSCALL WRITE - {}", other),
                                }
                            }
                            num_bytes
                        } else {
                            -Error::ENXIO as usize
                        }
                    }
                    _ => -Error::ENXIO as usize,
                }
            }
        }
        None => return -Error::EBADF as usize,
    }
}

pub fn read(trapframe: &TrapFrame) -> usize {
    let fd = trapframe.a0;
    let num_bytes = trapframe.a2;

    let buffer = unsafe {
        slice::from_raw_parts_mut(
            translate_virtual_address(trapframe.page_table, trapframe.a1)
                .unwrap_or_else(|e| panic!("READ FAILED: {}", e)) as *mut u8,
            num_bytes,
        )
    };

    if fd == STDIN {
        riscv::interrupt::supervisor::disable();

        unsafe {
            uart::READ = true;
            uart::INPUT_BUFFER.clear();
        }

        unsafe {
            riscv::interrupt::supervisor::enable();
        }

        let mut read = 0;

        loop {
            riscv::interrupt::supervisor::disable();
            if let Some(ch) = unsafe { INPUT_BUFFER.dequeue() } {
                if ch == '\n' as u8 {
                    if read < num_bytes {
                        buffer[read] = '\n' as u8;
                        read += 1;
                    }
                    break;
                } else if ch == 0x7f || ch == 0x08 {
                    if read > 0 {
                        read -= 1;
                    }
                } else {
                    if read < num_bytes {
                        buffer[read] = ch;
                        read += 1;
                    }
                }
            }

            unsafe {
                riscv::interrupt::supervisor::enable();
            }
        }

        buffer[read..num_bytes].fill(0);

        riscv::interrupt::supervisor::disable();
        unsafe {
            READ = false;
        }
        unsafe {
            riscv::interrupt::supervisor::enable();
        }

        return read;
    } else if fd == STDOUT || fd == STDERR {
        return -Error::ENXIO as usize;
    }

    match unsafe { FILES.get(&fd) } {
        Some(file) => match &*file.file_type.borrow() {
            FileType::Pipe(pipe) => {
                buffer.copy_from_slice(&pipe.read(num_bytes));
                num_bytes
            }
            FileType::INode {
                inode,
                offset,
                append,
            } => {
                if *offset > inode.size.get() {
                    return 0;
                }

                let data = match read_inode_data(inode, *offset, num_bytes, true, &DEVICE) {
                    Ok(data) => data,
                    Err(e) if e == crate::error::Error::ReadBeyondEOF => {
                        return -Error::EOVERFLOW as usize;
                    }
                    Err(e) => panic!("FILE READ - {:?}", e),
                };
                buffer.copy_from_slice(&data);
                return num_bytes;
            }
            FileType::Device { inode: _, major: _ } | FileType::Free => {
                panic!("io::read CALLED ON A DEVICE OR FREE INODE");
            }
        },
        None => return -Error::EBADF as usize,
    }
}

pub fn close(trapframe: &TrapFrame) -> usize {
    let fd = trapframe.a0;

    match unsafe { FILES.remove(&fd) } {
        Some(file) => match &*file.file_type.borrow() {
            FileType::INode {
                inode,
                offset,
                append,
            } => {
                flush_data_blocks(&DEVICE, true);
                flush_inodes(&DEVICE).unwrap();

                if inode.links.get() == 0 {
                    todo!();
                }
            }
            _ => (),
        },
        None => {}
    }

    0
}

pub fn lseek(trapframe: &TrapFrame) -> usize {
    let fd = trapframe.a0;
    let new_offset = trapframe.a1 as isize;

    let whence = trapframe.a2;

    match unsafe { FILES.get(&fd) } {
        Some(file) => match &mut *file.file_type.borrow_mut() {
            FileType::Pipe(_) => return -Error::EPIPE as usize,
            FileType::INode {
                inode,
                offset,
                append: _,
            } => {
                if new_offset < 0 {
                    if whence == 0 {
                        // offset can't be negative when whence is SEEK_SET
                        return -Error::EINVAL as usize;
                    }

                    if (new_offset).abs_diff(-(inode.size.get() as isize)) > inode.size.get() {
                        return -Error::EINVAL as usize;
                    }
                }
                match whence {
                    0 => *offset = new_offset as usize, // SEEK_SET
                    1 => *offset = (*offset as isize + new_offset) as usize, // SEEK_CUR
                    2 => *offset = (inode.size.get() as isize + new_offset) as usize, // SEEK_END
                    _ => return -Error::EINVAL as usize,
                }
                *offset
            }
            _ => return -Error::ENXIO as usize,
        },
        None => return -Error::EBADF as usize,
    }
}
