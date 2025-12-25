use alloc::vec;
use alloc::vec::Vec;

use crate::Storage;
use crate::fs::sfs::BLOCK_SIZE;

pub static mut RAM_DISK: Vec<u8> = Vec::new();

pub struct RamDisk;

impl Storage for RamDisk {
    fn initialise(&self) {
        unsafe {
            RAM_DISK = vec![0; 104857600];
        }
    }

    fn read_blocks(&self, start_block: usize, block_count: usize) -> Vec<u8> {
        let mut buffer = vec![0; block_count * BLOCK_SIZE];

        let mut j = 0;
        for i in start_block * BLOCK_SIZE..(start_block + block_count) * BLOCK_SIZE {
            buffer[j] = unsafe { RAM_DISK[i] };
            j += 1;
        }

        buffer
    }

    fn write_blocks(&self, start_block: usize, block_count: usize, buffer: &[u8]) {
        assert!(block_count * BLOCK_SIZE >= buffer.len());

        for i in 0..buffer.len() {
            unsafe {
                RAM_DISK[start_block * BLOCK_SIZE + i] = buffer[i];
            }
        }
    }
}
