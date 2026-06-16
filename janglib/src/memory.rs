use core::slice;

use alloc::string::String;
use hal::interrupts::SyscallArgs;

use crate::{
    Syscall,
    error::{Error, Result},
};

pub struct MemorySliceInner<const WRITABLE: bool, const SIZE: usize> {
    start: usize,
    page_table: usize,
}

pub enum MemorySlice<const WRITABLE: bool, const SIZE: usize> {
    User(MemorySliceInner<WRITABLE, SIZE>),
    Kernel(MemorySliceInner<WRITABLE, SIZE>),
}

impl<const SIZE: usize> MemorySlice<true, SIZE> {
    pub fn write(&mut self, buf: &[u8; SIZE]) -> Result<usize> {
        let start = match self {
            MemorySlice::User(memory_slice_inner) => {
                let mut args = SyscallArgs::default();
                args.0 = Syscall::ToPhysicalAddress as usize;
                args.1 = memory_slice_inner.start;
                args.2 = memory_slice_inner.page_table;
                hal::interrupts::InterruptHandling::make_sycall(args).unwrap()
            }
            MemorySlice::Kernel(memory_slice_inner) => memory_slice_inner.start,
        };

        let slice = unsafe { slice::from_raw_parts_mut(start as *mut u8, SIZE) };
        slice.copy_from_slice(buf);
        Ok(SIZE)
    }
}

pub fn want_memory(size: usize) -> Result<(usize, usize)> {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::WantMemory as usize;
    args.1 = (size + hal::constants::PAGE_SIZE - 1) / hal::constants::PAGE_SIZE;
    hal::interrupts::InterruptHandling::make_sycall(args)
        .map(|start| (start, args.1))
        .map_err(|e| Error::MemoryNotAvailable)
}
