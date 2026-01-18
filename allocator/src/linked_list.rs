use core::ptr::NonNull;

pub const MAGIC_1: u128 = 211482366920938462397425787431768211125;
pub const MAGIC_2: u128 = 192137983263256483582376453435164352209;

pub struct Node {
    pub magic_1: u128,
    pub next: Option<NonNull<Node>>,
    pub prev: Option<NonNull<Node>>,
    pub size: usize,
    pub id: u128,
    pub magic_2: u128,
}

pub fn check_magic(node: NonNull<Node>) {
    unsafe {
        debug_assert_eq!((*node.as_ptr()).magic_1, MAGIC_1, "MEMORY CORRUPT");
        debug_assert_eq!((*node.as_ptr()).magic_2, MAGIC_2, "MEMORY CORRUPT");
    }
}

pub fn generate_node_id() -> u128 {
    static mut NODE_ID: u128 = 0;

    unsafe {
        NODE_ID += 1;
        NODE_ID - 1
    }
}

pub struct LinkedList {
    pub head: Option<NonNull<Node>>,
}

impl LinkedList {
    pub fn push_front(&mut self, other: NonNull<Node>) {
        let mut node = self.head;
        let mut prev = None;

        while let Some(n) = node {
            check_magic(n);

            node = unsafe { (*n.as_ptr()).next };
            prev = Some(n);
        }

        if let Some(prev) = prev {
            check_magic(prev);

            unsafe {
                (*prev.as_ptr()).next = Some(other);
                (*other.as_ptr()).prev = Some(prev);
            }
        } else {
            self.head = Some(other);
        }
    }

    pub fn pop_front(&mut self) -> Option<NonNull<Node>> {
        if let Some(node) = self.head {
            check_magic(node);

            let next = unsafe { (*node.as_ptr()).next };
            self.head = next;

            if let Some(next) = next {
                check_magic(next);

                unsafe {
                    (*next.as_ptr()).prev = None;
                }
            }

            return Some(node);
        }

        None
    }

    pub fn remove(&mut self, node_id: u128) -> NonNull<Node> {
        let mut curr = self.head;

        while let Some(node) = curr {
            check_magic(node);

            if unsafe { (*node.as_ptr()).id } == node_id {
                if let Some(prev) = unsafe { (*node.as_ptr()).prev } {
                    unsafe {
                        (*prev.as_ptr()).next = (*node.as_ptr()).next;
                    }
                }

                if let Some(next) = unsafe { (*node.as_ptr()).next } {
                    unsafe {
                        (*next.as_ptr()).prev = (*node.as_ptr()).prev;
                    }
                }

                return node;
            }

            unsafe {
                curr = (*node.as_ptr()).next;
            }
        }

        panic!("Allocation Error: No such node");
    }
}
