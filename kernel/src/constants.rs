use hal::constants::PAGE_SIZE;

pub const TIME_SLICE: usize = 100000;

pub static mut KERNEL_END: usize = 0;
pub static mut END_OF_KERNEL_TEXT: usize = 0; // re-assign later in main::_start
pub static mut TRAMPOLINE_CODE_ADDRESS: usize = 0; // // re-assign later in main::_start
pub static mut KERNEL_START: usize = 0;
pub const KERNEL_HEAP_SIZE: usize = 524288000;

#[allow(non_upper_case_globals)]
pub const Sv48: usize = 9 << 60;

// Memory-Mapped Register Address
pub const PLIC: usize = 0xc000000;
pub const PLIC_SIZE: usize = 0x600000;

// UART
pub const UART0: usize = 0x10000000;

pub static mut KERNEL_PAGE_TABLE: usize = 0;

pub const MAXIMUM_PROCESS: usize = 64;

pub const RAM_STOP: usize = 0x80000000 + 0x100000000;

pub static mut TRAMPOLINE_OFFSET: usize = 0;
