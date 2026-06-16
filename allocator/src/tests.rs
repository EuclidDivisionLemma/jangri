extern crate std;
use std::alloc::alloc;
use std::boxed::Box;

use core::{alloc::Layout, ptr::NonNull};

use crate::{
    PAGE_SIZE, PageAllocator,
    linked_list::{LinkedList, MAGIC_1, MAGIC_2, Node, generate_node_id},
};

#[test]
fn test_linked_list() {
    let mut node = Box::new(Node {
        magic_1: MAGIC_1,
        next: None,
        prev: None,
        size: 1209,
        id: generate_node_id(),
        magic_2: MAGIC_2,
    });

    let mut list = LinkedList {
        head: unsafe { Some(NonNull::new_unchecked(&raw mut *node.as_mut())) },
    };

    let mut c1 = Box::new(Node {
        magic_1: MAGIC_1,
        next: None,
        prev: None,
        size: 120,
        id: generate_node_id(),
        magic_2: MAGIC_2,
    });

    let mut c2 = Box::new(Node {
        magic_1: MAGIC_1,
        next: None,
        prev: None,
        size: 988,
        id: generate_node_id(),
        magic_2: MAGIC_2,
    });

    unsafe {
        list.push_front(NonNull::new_unchecked(&raw mut *c1.as_mut()));
        list.push_front(NonNull::new_unchecked(&raw mut *c2.as_mut()));

        assert_eq!(
            (*(*list.head.unwrap().as_ptr()).next.unwrap().as_ptr()).size,
            120
        );

        assert_eq!(
            (*(*(*list.head.unwrap().as_ptr()).next.unwrap().as_ptr())
                .prev
                .unwrap()
                .as_ptr())
            .size,
            1209
        );

        let chead = list.pop_front().unwrap();
        let cc1 = list.pop_front().unwrap();
        let cc2 = list.pop_front().unwrap();

        assert_eq!((*chead.as_ptr()).size, 1209);
        assert_eq!((*cc1.as_ptr()).size, 120);
        assert_eq!((*cc2.as_ptr()).size, 988);
    }

    let n1 = unsafe {
        std::alloc::alloc(Layout::from_size_align(2 * size_of::<Node>(), 32).unwrap()) as *mut Node
    };

    let n2 = n1.map_addr(|addr| addr + size_of::<Node>());

    unsafe {
        *n2 = Node {
            magic_1: MAGIC_1,
            next: None,
            prev: None,
            size: 100,
            id: generate_node_id(),
            magic_2: MAGIC_2,
        };

        *n1 = Node {
            magic_1: MAGIC_1,
            next: None,
            prev: None,
            size: 100,
            id: generate_node_id(),
            magic_2: MAGIC_2,
        };

        let n3 = &raw mut *&mut Node {
            magic_1: MAGIC_1,
            next: None,
            prev: None,
            size: 250,
            id: generate_node_id(),
            magic_2: MAGIC_2,
        };

        list.head = None;

        list.push_front(NonNull::new(n1).unwrap());
        list.push_front(NonNull::new(n3).unwrap());
        list.push_front(NonNull::new(n2).unwrap());

        assert_eq!((*list.head.unwrap().as_ptr()).id, (*n1).id);
        assert_eq!(
            (*(*list.head.unwrap().as_ptr()).next.unwrap().as_ptr()).id,
            (*n3).id
        );

        list.remove((*n3).id);

        assert_eq!((*list.head.unwrap().as_ptr()).id, (*n1).id);
        assert_eq!(
            (*(*list.head.unwrap().as_ptr()).next.unwrap().as_ptr()).id,
            (*n2).id
        );
    }
}

#[test]
#[should_panic(expected = "Allocation Error: Size (20) is not a power of two")]
fn test_allocator() {
    let heap = unsafe { alloc(Layout::from_size_align(256 * PAGE_SIZE, PAGE_SIZE).unwrap()) };
    let mut allocator = PageAllocator::new(
        &|_| Err(hal::error::Error::MemoryNotAvailable),
        heap.addr(),
        heap.addr() + 256 * PAGE_SIZE,
    );

    let four_pages = allocator.allocate(4 * PAGE_SIZE).unwrap() as *mut usize;
    let sixteen_pages = allocator.allocate(16 * PAGE_SIZE).unwrap() as *mut usize;
    let sixty_four_pages = allocator.allocate(64 * PAGE_SIZE).unwrap() as *mut usize;
    let one_hundred_twenty_eight_pages = allocator.allocate(128 * PAGE_SIZE).unwrap() as *mut usize;
    let thirty_two_pages = allocator.allocate(32 * PAGE_SIZE).unwrap() as *mut usize;
    let eight_pages = allocator.allocate(8 * PAGE_SIZE).unwrap() as *mut usize;
    let another_four_pages = allocator.allocate(4 * PAGE_SIZE).unwrap() as *mut usize;

    unsafe {
        *sixteen_pages = 9855658;
        *four_pages = 12274387;
        *sixty_four_pages = 117868;
        *one_hundred_twenty_eight_pages = 123456;
        *thirty_two_pages = 32456;
        *eight_pages = 909878;
        *another_four_pages = 92843;

        assert_eq!(*sixteen_pages, 9855658);
        assert_eq!(*four_pages, 12274387);
        assert_eq!(*sixty_four_pages, 117868);
        assert_eq!(*one_hundred_twenty_eight_pages, 123456);
        assert_eq!(*thirty_two_pages, 32456);
        assert_eq!(*eight_pages, 909878);
        assert_eq!(*another_four_pages, 92843);
        assert_eq!(*sixteen_pages, 9855658);
    }

    assert!(allocator.allocate(PAGE_SIZE).is_err());

    allocator.deallocate(one_hundred_twenty_eight_pages.addr(), 128 * PAGE_SIZE);

    let some_more_pages = allocator.allocate(4 * PAGE_SIZE).unwrap() as *mut usize;
    unsafe {
        *some_more_pages = 983645;
        assert_eq!(*some_more_pages, 983645);
    }

    assert!(allocator.allocate(20).is_err());
    allocator.deallocate(four_pages.addr(), 4 * PAGE_SIZE);
    allocator.deallocate(sixteen_pages.addr(), 16 * PAGE_SIZE);
    allocator.deallocate(sixty_four_pages.addr(), 64 * PAGE_SIZE);
    allocator.deallocate(thirty_two_pages.addr(), 32 * PAGE_SIZE);
    allocator.deallocate(eight_pages.addr(), 8 * PAGE_SIZE);
    allocator.deallocate(another_four_pages.addr(), 4 * PAGE_SIZE);
    allocator.deallocate(some_more_pages.addr(), 4 * PAGE_SIZE);
}
