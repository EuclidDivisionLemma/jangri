#[allow(unused_imports)]
pub use crate::vm::constants::*;

pub const TRAMPOLINE: usize = 0xfffffffffffff000;
pub const TRAPFRAME: usize = TRAMPOLINE - 2 * PAGE_SIZE;

/// Kernel-User Communication Page
pub const KUCOM_PAGE: usize = TRAPFRAME - 2 * PAGE_SIZE;

pub const STACK_START: usize = KUCOM_PAGE - PAGE_SIZE - 8 * PAGE_SIZE;
pub const STACK_GUARD: usize = STACK_START - PAGE_SIZE;
pub const STACK_PAGES: usize = 8;
pub const TIME_SLICE: usize = 100000;
