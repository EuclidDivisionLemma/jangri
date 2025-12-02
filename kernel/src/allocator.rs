use core::{alloc::GlobalAlloc, cell::RefCell, f64::math::ceil, ptr::null_mut};

use crate::constants::{KERNEL_END, MEM_SIZE, PAGE_SIZE};

pub struct Allocator {
    bitmap: RefCell<[bool; MEM_SIZE / PAGE_SIZE]>,
}

#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator {
    bitmap: RefCell::new([false; MEM_SIZE / PAGE_SIZE]),
};

fn find_contiguous(num_pages: usize) -> Option<usize> {
    let mut allocator = ALLOCATOR.bitmap.borrow_mut();

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
            num_pages = ceil(layout.size() as f64 / PAGE_SIZE as f64) as usize;
        }

        match find_contiguous(num_pages) {
            Some(start) => (unsafe { KERNEL_END } + (start * PAGE_SIZE)) as *mut u8,
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut num_pages = 1;

        if layout.size() > PAGE_SIZE {
            num_pages = ceil(layout.size() as f64 / PAGE_SIZE as f64) as usize;
        }

        let index = (ptr.addr() - unsafe { KERNEL_END }) / PAGE_SIZE;

        for i in index..index + num_pages {
            let mut bitmap = self.bitmap.borrow_mut();
            bitmap[i] = false;
        }
    }
}

// THE OS runs on one CPU
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}
