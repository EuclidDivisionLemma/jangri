use core::num::{NonZero, NonZeroUsize};

pub const PAGE_SIZE: usize = 4096;
pub const MEM_SIZE: usize = 0x80000000;
pub const TIME_SLICE: usize = 100000;

pub static mut KERNEL_END: usize = 0;
pub static mut END_OF_KERNEL_TEXT: usize = 0; // re-assign later in main::_start
pub static mut TRAMPOLINE_CODE_ADDRESS: usize = 0; // // re-assign later in main::_start
pub static mut KERNEL_START: usize = 0;

pub const STACK_START: usize = TRAMPOLINE - 8 * PAGE_SIZE;

pub const MAX_VA: usize = 0xffffffffffffffff;
pub const VALID_BIT: usize = 1;

#[allow(non_upper_case_globals)]
pub const Sv48: usize = 9 << 60;

// Memory-Mapped Register Address
pub const PLIC: usize = 0xc000000;
pub const PLIC_SIZE: usize = 0x600000;
pub const VIRTIO_MMIO_DISK: usize = 0x10001000;
pub const VIRTIO_MMIO_DISK_SIZE: usize = 0x1000;

// UART
pub const UART0: usize = 0x10000000;
pub const UART_TEXT_BUFFER_SIZE: usize = 1000;

pub static mut KERNEL_PAGE_TABLE: usize = 0;

// Page Permissions
pub const READ_WRITE: usize = 0b0110;
pub const READ_ONLY: usize = 0b10;
pub const WRITE_ONLY: usize = 0b100;
pub const EXECUTE_ONLY: usize = 0b1000;
pub const USER_MODE: usize = 0b10000;
pub const READ_EXECUTE: usize = 0b1010;
pub const READ_WRITE_EXECUTE: usize = 0b1110;

pub const MAXIMUM_PROCESS: usize = 64;

pub const TRAMPOLINE: usize = 0xfffffffffffff000;
pub const TRAPFRAME: usize = TRAMPOLINE - PAGE_SIZE;

pub const RAM_STOP: usize = 0x80000000 + 2_14_74_83_648;

pub const TIMER_EXTENION_ID: usize = 0x54494D45;

pub const STACK_PAGES: usize = 6;

pub static mut TRAMPOLINE_OFFSET: usize = 0;

pub const UART_ID: usize = 10;
pub const UART_PRIORITY_ADDRESS: usize = PLIC + UART_ID * 4;
pub const PLIC_S_MODE_ENABLE: usize = PLIC + 0x2080;
pub const PLIC_S_MODE_THRESHOLD: usize = PLIC + 0x201000;
pub const PLIC_S_MODE_CLAIM: usize = PLIC + 0x201004;

pub const ROOT_INODE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1) };
