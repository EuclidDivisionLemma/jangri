use core::{
    fmt::Debug,
    ptr::{NonNull, read, write_volatile},
};

extern crate alloc;

use hal::constants::PAGE_SIZE;

use crate::{
    bitmap::Bitmap,
    linked_list::{LinkedList, MAGIC_1, MAGIC_2, Node},
};

const MAX_ALLOC: usize = 10485760; // 10 MiB

pub const NUM_MULTIPLES: usize = 2560;

pub struct PageAllocator {
    bitmaps: Bitmap,
    memory_start: usize,
    memory_end: usize,
    current_size: usize,
    current_start: usize,
    list: [LinkedList; NUM_MULTIPLES],
}

impl Debug for PageAllocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&alloc::format!(
            "Global Page Allocator managing memory
            between {:#x} and {:#x}",
            self.memory_start,
            self.memory_end,
        ))
    }
}

impl PageAllocator {
    pub const fn new(start: usize, end: usize) -> Self {
        if start == 0 {
            panic!("start is NULL");
        }

        let size = end
            .checked_sub(start)
            .expect("end must be greater than start");

        Self {
            bitmaps: Bitmap::new(),
            memory_start: start,
            memory_end: end,
            list: [LinkedList { head: None }; NUM_MULTIPLES],
            current_size: size,
            current_start: start,
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        if size < PAGE_SIZE {
            panic!(
                "Allocation Error: Size ({}) is less than minimum allocable size (4096)",
                size
            );
        }

        assert!(
            size % PAGE_SIZE == 0,
            "size is not a multiple of PAGE_SIZE which is 4KiB"
        );

        let multiple = size / PAGE_SIZE;
        if let Some(v) = self.bitmaps.first_available(multiple) {
            let list = self.list.get_mut(v - 1).unwrap();
            let block = list.pop_front().unwrap();
            if list.head.is_none() {
                self.bitmaps.mark_unavailable(multiple);
            }

            if v == multiple {
                return Some(block.addr().get());
            }

            let needed = multiple * PAGE_SIZE;
            let not_needed = v * PAGE_SIZE - needed;
            let not_needed_multiple = not_needed / PAGE_SIZE;

            let node = Node::new(None, None, not_needed);
            let not_needed_addr = (block.addr().get() + needed) as *mut Node;
            unsafe {
                write_volatile(not_needed_addr, node);
            }
            self.list
                .get_mut(not_needed_multiple - 1)
                .unwrap()
                .push_front(NonNull::new(not_needed_addr).unwrap());
            self.bitmaps.mark_available(not_needed_multiple);
            return Some(block.addr().get());
        }

        if self.current_start < self.memory_end {
            assert!(self.current_size != 0);
            let old_start = self.current_start;
            self.current_start += size;
            self.current_size -= size;
            return Some(old_start);
        }

        None
    }

    pub fn deallocate(&mut self, start: usize, mut size: usize) {
        assert!(size % PAGE_SIZE == 0);
        assert!(size >= PAGE_SIZE && size <= MAX_ALLOC);

        let next = (start + size) as *const Node;
        unsafe {
            if read(next).magic_1 == MAGIC_1 && read(next).magic_2 == MAGIC_2 {
                size += read(next).size;
                let list = self
                    .list
                    .get_mut((read(next).size / PAGE_SIZE) - 1)
                    .unwrap();
                list.remove(read(next).id);
                if list.head.is_none() {
                    self.bitmaps.mark_unavailable(read(next).size / PAGE_SIZE);
                }
            }
        }

        let node = Node::new(None, None, size);
        let node_addr = start as *mut Node;
        unsafe {
            write_volatile(node_addr, node);
        }
        self.list
            .get_mut((size / PAGE_SIZE) - 1)
            .unwrap()
            .push_front(NonNull::new(node_addr).unwrap());
        self.bitmaps.mark_available(size / PAGE_SIZE);
    }
}
