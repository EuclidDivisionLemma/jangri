#![no_std]

mod linked_list;
mod page_allocator;

#[cfg(test)]
mod tests;

use core::sync::atomic::AtomicUsize;

pub use page_allocator::PageAllocator;

pub const PAGE_SIZE: usize = 4096;

pub static ALLOC: AtomicUsize = AtomicUsize::new(0);
