#![no_std]

mod linked_list;
mod page_allocator;

#[cfg(test)]
mod tests;

pub use page_allocator::PageAllocator;

pub const PAGE_SIZE: usize = 4096;
