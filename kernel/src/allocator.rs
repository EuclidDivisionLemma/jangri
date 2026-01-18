use core::{
    alloc::{GlobalAlloc, Layout},
    cell::{Cell, RefCell},
    ptr::{null_mut, write_bytes},
};

use crate::{
    constants::{KERNEL_END, KERNEL_START, MEM_SIZE, PAGE_SIZE},
    error::Result,
};

#[repr(C)]
pub struct Allocator {
    bitmap: [bool; MEM_SIZE / PAGE_SIZE],
}

#[global_allocator]
pub static mut ALLOCATOR: Allocator = Allocator {
    bitmap: [false; MEM_SIZE / PAGE_SIZE],
};

fn find_contiguous(num_pages: usize) -> Option<usize> {
    let allocator = unsafe { &mut ALLOCATOR.bitmap };

    let mut count = 0;
    let mut start: isize = -1;

    for i in 0..allocator.len() {
        if count < num_pages {
            if allocator[i] == false {
                if count == 0 {
                    start = i as isize;
                }
                count += 1;
            } else {
                start = -1;
                count = 0;
            }
        } else {
            break;
        }
    }

    if start == -1 {
        None
    } else {
        if count != num_pages {
            return None;
        }
        for i in start as usize..(start as usize + num_pages) {
            allocator[i] = true;
        }
        Some(start as usize)
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut num_pages = 1;
        if layout.size() > PAGE_SIZE {
            num_pages = (layout.size() + PAGE_SIZE - 1) / PAGE_SIZE;
        }

        match find_contiguous(num_pages) {
            Some(start) => {
                let address = (unsafe { KERNEL_END } + (start * PAGE_SIZE)) as *mut u8;

                if address.addr() + num_pages * PAGE_SIZE > (unsafe { KERNEL_START } + MEM_SIZE) {
                    null_mut()
                } else {
                    address
                }
            }
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut num_pages = 1;

        if layout.size() > PAGE_SIZE {
            num_pages = (layout.size() + PAGE_SIZE - 1) / PAGE_SIZE;
        }

        let index = (ptr.addr() - unsafe { KERNEL_END }) / PAGE_SIZE;

        for i in index..index + num_pages {
            let bitmap = self.bitmap.as_ptr().addr() as *mut bool;
            unsafe { *bitmap.offset(i as isize) = false };
        }
    }
}

// THE OS runs on one CPU
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

pub fn allocate(num_pages: usize) -> Result<usize> {
    unsafe {
        let layout = Layout::from_size_align_unchecked(4096 * num_pages, 4096);
        let ptr = ALLOCATOR.alloc_zeroed(layout);
        if ptr == null_mut() {
            Err(crate::error::Error::NoFreePage)
        } else {
            Ok(ptr.addr())
        }
    }
}

pub fn deallocate(pa: usize) {
    // pa is the same as va, as pages are identity mapped
    assert!(pa % PAGE_SIZE == 0, "MISALIGNED VA WHILE DEALLOCATING");

    let allocator = unsafe { &mut ALLOCATOR.bitmap };
    allocator[(pa - unsafe { KERNEL_END }) / PAGE_SIZE] = false;
}
