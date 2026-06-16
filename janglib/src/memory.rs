use core::slice;

use hal::{
    error::{Error, Result},
    interrupts::InterruptHandling,
};

use crate::Syscall;

pub struct UserMemorySlice<const WRITABLE: bool, F: Fn(usize, usize) -> Result<usize>> {
    start: usize,
    page_table: usize,
    size: usize,
    translate: F,
}

impl<const WRITABLE: bool, F: Fn(usize, usize) -> Result<usize>> UserMemorySlice<WRITABLE, F> {
    pub const fn new(start: usize, page_table: usize, size: usize, translate: F) -> Self {
        UserMemorySlice {
            start,
            page_table,
            size,
            translate,
        }
    }
}

impl<'a, const WRITABLE: bool, F: Fn(usize, usize) -> Result<usize>> UserMemorySlice<WRITABLE, F> {
    pub fn read(&self) -> &'a [u8] {
        let start = (self.translate)(self.start, self.page_table).unwrap();
        unsafe { slice::from_raw_parts(start as *const u8, self.size) }
    }
}

impl<F: Fn(usize, usize) -> Result<usize>> UserMemorySlice<true, F> {
    pub fn write(&mut self, bytes: &[u8]) {
        let start = (self.translate)(self.start, self.page_table).unwrap();
        let mem = unsafe { slice::from_raw_parts_mut(start as *mut u8, self.size) };
        mem.copy_from_slice(bytes);
    }
}

pub fn want_memory<I: InterruptHandling>(size: usize) -> Result<(usize, usize)> {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::WantMemory as usize;
    args.1 = (size + hal::constants::PAGE_SIZE - 1) / hal::constants::PAGE_SIZE;
    I::make_sycall(args)
        .map(|start| (start, args.1))
        .map_err(|_| Error::MemoryNotAvailable)
}
