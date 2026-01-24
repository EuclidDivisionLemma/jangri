use alloc::vec::Vec;

pub mod ram_disk;
pub mod virtio_disk;

pub trait Storage {
    fn initialise(&self);
    fn read_blocks(&self, start_block: usize, block_count: usize) -> Vec<u8>;
    fn write_blocks(&self, start_block: usize, block_count: usize, buffer: &[u8]);
}
