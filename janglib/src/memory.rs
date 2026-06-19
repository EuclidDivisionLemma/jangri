use core::{alloc::GlobalAlloc, arch::asm, ptr::null_mut, slice};

use alloc::vec::Vec;
use hal::{
    error::{Error, Result},
    interrupts::InterruptHandling,
    vm::align_to_page_size,
};

use crate::{KUCOM_PAGE, Syscall, SyscallResult, get_result, make_syscall, write_syscall};

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

    pub fn read_to_vec(&self) -> Vec<u8> {
        self.read().to_vec()
    }
}

impl<F: Fn(usize, usize) -> Result<usize>> UserMemorySlice<true, F> {
    pub fn write(&mut self, bytes: &[u8]) {
        let start = (self.translate)(self.start, self.page_table).unwrap();
        let mem =
            unsafe { slice::from_raw_parts_mut(start as *mut u8, self.size.min(bytes.len())) };
        mem.copy_from_slice(&bytes[0..self.size.min(bytes.len())]);
    }
}

pub fn want_memory(size: usize) -> Result<(usize, usize)> {
    let size = align_to_page_size(size);
    write_syscall(Syscall::WantMemory(size));
    make_syscall!(Syscall::WantMemory);
    check()
}
