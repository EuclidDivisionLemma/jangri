use core::{alloc::GlobalAlloc, cell::RefCell, f64::math::ceil, ptr::null_mut};

use crate::constants::{KERNEL_END, MEM_SIZE, PAGE_SIZE};

pub struct Allocator {
    bitmap: RefCell<[bool; MEM_SIZE / PAGE_SIZE]>,
}

#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator {
    bitmap: RefCell::new([false; MEM_SIZE / PAGE_SIZE]),
};

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut num_pages = 1;
        if layout.size() > PAGE_SIZE {
            num_pages = ceil(layout.size() as f64 / PAGE_SIZE as f64) as usize;
        }
        let mut head = None;

        for _ in 0..num_pages {
            for j in 0..self.bitmap.borrow().len() {
                if (*self.bitmap.borrow())[j] == false {
                    (*self.bitmap.borrow_mut())[j] = true;
                    if let None = head {
                        head = Some((unsafe { KERNEL_END } + (j * PAGE_SIZE)) as *mut u8);
                    }
                }
            }
        }
        head.unwrap_or(null_mut())
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
