#![no_std]

mod bitmap;
mod linked_list;
mod page_allocator;

#[cfg(test)]
mod tests;

use core::sync::atomic::AtomicUsize;

pub use page_allocator::MAX_ALLOC;
pub use page_allocator::PageAllocator;

pub static ALLOC: AtomicUsize = AtomicUsize::new(0);
