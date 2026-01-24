use core::{
    array::from_fn,
    fmt::Debug,
    ptr::{NonNull, write_bytes, write_volatile},
};

extern crate alloc;

use anyhow::{Result, bail};

use crate::{
    PAGE_SIZE,
    linked_list::{LinkedList, MAGIC_1, MAGIC_2, Node, generate_node_id},
};

const MAX_ALLOC: usize = 1073741824;
const MIN_ALLOC: usize = PAGE_SIZE;

const MAX_ORDER: usize = 30;
const MIN_ORDER: usize = 12;

const BUCKET_COUNT: usize = MAX_ORDER - MIN_ORDER;

pub struct PageAllocator {
    buckets: [LinkedList; BUCKET_COUNT],
    memory_start: usize,
    memory_end: usize,
    current_size: usize,
    current_start: usize,
    evict: &'static dyn Fn(usize) -> Result<usize>,
}

impl Debug for PageAllocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&alloc::format!(
            "Global Page Allocator managing memory
            between {} and {} with currently carveable memory {}",
            self.memory_start,
            self.memory_end,
            self.current_size
        ))
    }
}

fn index_from_order(order: usize) -> usize {
    debug_assert!(order <= 30, "Order out of range");
    order - 12
}

fn order_from_size(size: usize) -> usize {
    debug_assert!(size <= MAX_ALLOC, "Allocation Size > 1GiB");
    debug_assert!(size.is_power_of_two());
    size.next_power_of_two().ilog2() as usize
}

impl PageAllocator {
    pub fn new(evict: &'static dyn Fn(usize) -> Result<usize>, start: usize, end: usize) -> Self {
        Self {
            buckets: from_fn(|_| LinkedList { head: None }),
            memory_start: start,
            memory_end: end,
            current_size: end - start,
            current_start: start,
            evict,
        }
    }

    fn get_best_fit(&mut self, size: usize) -> Option<NonNull<Node>> {
        assert!(size.is_power_of_two() && size >= PAGE_SIZE);
        let order = size.ilog2() as usize;

        for order in order..MAX_ORDER {
            if self.buckets[index_from_order(order)].head.is_some() {
                return self.buckets[index_from_order(order)].pop_front();
            }
        }

        None
    }

    fn find_buddy(&self, this: NonNull<Node>) -> Option<NonNull<Node>> {
        let size = unsafe { (*this.as_ptr()).size };

        if this.as_ptr().addr() + size * 2 >= self.memory_end {
            return None;
        }

        let buddy = NonNull::new(this.as_ptr().map_addr(|addr| addr + size))?;

        if unsafe { (*buddy.as_ptr()).magic_1 } != MAGIC_1
            || unsafe { (*buddy.as_ptr()).size != (*this.as_ptr()).size }
        {
            return None;
        }

        Some(buddy)
    }

    pub fn allocate(&mut self, size: usize) -> Result<usize> {
        // The caller must ensure that the size is a power of two.
        // This prevents a non-power-of-two size from being passed
        // inadvertantly, causing a larger size to be allocated.

        if !size.is_power_of_two() {
            bail!("Allocation Error: Size ({}) is not a power of two", size);
        }

        if size < PAGE_SIZE {
            bail!(
                "Allocation Error: Size ({}) is less than minimum allocable size (4096)",
                size
            );
        }

        assert!(size.is_power_of_two() && size >= PAGE_SIZE);

        if size < MIN_ALLOC {
            bail!("Allocation Error: Minimum allocation size is 4KiB");
        }

        let best_block = self.get_best_fit(size);

        if let Some(mut best_block) = best_block {
            let mut block_size = unsafe { (*best_block.as_ptr()).size };

            while size < block_size {
                unsafe {
                    (*best_block.as_ptr()).size /= 2;
                }

                block_size /= 2;

                let buddy = NonNull::new(best_block.as_ptr().map_addr(|addr| addr + block_size))
                    .expect("Allocation Error: Buddy Null in allocate");

                unsafe {
                    (*buddy.as_ptr()) = Node {
                        magic_1: MAGIC_1,
                        next: None,
                        prev: None,
                        size: block_size,
                        id: generate_node_id(),
                        magic_2: MAGIC_2,
                    };
                }

                let order = order_from_size(block_size);
                self.buckets[index_from_order(order)].push_front(buddy);
                self.buckets[index_from_order(order)].push_front(best_block);

                best_block = self.buckets[index_from_order(order)].pop_front().unwrap();
            }

            unsafe {
                write_bytes(best_block.as_ptr() as *mut u8, 0, size);
            }

            return Ok(best_block.as_ptr() as usize);
        }

        if self.current_size == 0 || size > self.current_size {
            match (self.evict)(size / PAGE_SIZE) {
                Ok(addr) => return Ok(addr),
                Err(e) => bail!(
                    "Allocation Error: No Free Page; caused by: {}; {:?}",
                    e,
                    self
                ),
            }
        }

        let carved_memory = self.current_start;
        self.current_start += size;
        self.current_size -= size;

        unsafe {
            write_bytes(carved_memory as *mut u8, 0, size);
        }

        Ok(carved_memory)
    }

    pub fn deallocate(&mut self, start: usize, size: usize) {
        // Since all allocations were powers of two, all deallocations must also be.
        // If not, it is a bug
        debug_assert!(size.is_power_of_two());

        if start + size == self.current_start {
            debug_assert!(self.current_start as i64 - size as i64 >= self.memory_start as i64);
            self.current_start -= size;
            self.current_size += size;
            return;
        }

        let order = order_from_size(size);
        let addr = NonNull::new(start as *mut Node)
            .expect("Allocation Error: Pointer null during deallocation");

        let node = Node {
            magic_1: MAGIC_1,
            next: None,
            prev: None,
            size,
            id: generate_node_id(),
            magic_2: MAGIC_2,
        };

        unsafe {
            write_volatile(addr.as_ptr(), node);
        }

        self.buckets[index_from_order(order)].push_front(addr);

        let block = addr;
        let mut found = false;
        while let Some(buddy) = self.find_buddy(block) {
            found = true;
            self.buckets[index_from_order(order)].remove(unsafe { (*buddy.as_ptr()).id });
            unsafe {
                (*block.as_ptr()).size *= 2;
            }
        }

        if found {
            unsafe {
                (*block.as_ptr()).next = None;
                (*block.as_ptr()).prev = None;
            }

            let new_order = order_from_size(unsafe { (*block.as_ptr()).size });
            self.buckets[index_from_order(new_order)].push_front(block);
        }
    }

    pub fn get_carveable_memory(&self) -> usize {
        self.current_size
    }
}
