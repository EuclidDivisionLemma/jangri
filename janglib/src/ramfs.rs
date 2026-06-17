use core::ptr::slice_from_raw_parts;

use alloc::string::ToString;

const MAGIC: [u8; 8] = [0x93, 0x24, 0x72, 0x35, 0x32, 0x97, 0x05, 0x9];
const MAX_NAME_LENGTH: usize = 255;

#[derive(Debug, Clone, Copy)]
pub struct RamFsHeader {
    pub magic: [u8; 8],
    pub number_of_entries: [u8; 8],
}

#[derive(Debug, Clone, Copy)]
pub struct RamFsEntryHeader {
    pub magic: [u8; 8],
    pub name: [u8; MAX_NAME_LENGTH],
    pub size: [u8; 8],
}

impl RamFsHeader {
    pub fn new(number_of_entries: u64) -> Self {
        Self {
            magic: MAGIC,
            number_of_entries: number_of_entries.to_be_bytes(),
        }
    }

    pub fn number_of_entries(&self) -> u64 {
        u64::from_be_bytes(self.number_of_entries)
    }
}

impl RamFsEntryHeader {
    pub fn name(&self) -> &str {
        str::from_utf8(self.name.as_slice()).unwrap()
    }

    pub fn from_addr(start: usize) -> Self {
        let start = start as *const Self;
        assert!(unsafe { (*start).magic == MAGIC });
        unsafe { *start }
    }

    pub fn size(&self) -> u64 {
        u64::from_be_bytes(self.size)
    }

    pub fn new(name: &str, size: u64) -> Self {
        let mut name_buf = [0u8; MAX_NAME_LENGTH];
        name_buf[0..(MAX_NAME_LENGTH as u64).min(size) as usize]
            .copy_from_slice(&name.as_bytes()[0..(MAX_NAME_LENGTH as u64).min(size) as usize]);
        Self {
            magic: MAGIC,
            name: name_buf,
            size: size.to_be_bytes(),
        }
    }
}

pub struct RamFs {
    start: usize,
}

impl RamFs {
    pub fn from_addr(start: usize) -> Self {
        let start = start as *const u8;

        let mut num_buf = [0u8; 8];
        let number_of_entries = unsafe { &*slice_from_raw_parts(start.add(8), 8) };
        num_buf.copy_from_slice(number_of_entries);
        let number_of_entries = u64::from_be_bytes(num_buf);
        let start = unsafe { start.add(16) };
        assert!(!start.is_null());

        for _ in 0..number_of_entries {
            let _ = RamFsEntryHeader::from_addr(start.addr());
        }

        RamFs {
            start: unsafe { start.addr() },
        }
    }
}
