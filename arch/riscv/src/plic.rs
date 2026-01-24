use core::ptr::{read_volatile, write_volatile};

pub const PLIC: usize = 0xc000000;

pub const UART_ID: usize = 10;
pub const UART_PRIORITY_ADDRESS: usize = PLIC + UART_ID * 4;
pub const PLIC_S_MODE_ENABLE: usize = PLIC + 0x2080;
pub const PLIC_S_MODE_THRESHOLD: usize = PLIC + 0x201000;
pub const PLIC_S_MODE_CLAIM: usize = PLIC + 0x201004;

pub fn initialise() {
    unsafe {
        write_volatile(UART_PRIORITY_ADDRESS as *mut u32, 1);
        write_volatile(PLIC_S_MODE_ENABLE as *mut u32, 1 << UART_ID);
        write_volatile(PLIC_S_MODE_THRESHOLD as *mut u32, 0);
    }
}

pub fn claim() -> usize {
    unsafe { read_volatile(PLIC_S_MODE_CLAIM as *mut u32) as usize }
}

pub fn complete(id: usize) {
    unsafe {
        write_volatile(PLIC_S_MODE_CLAIM as *mut u32, id as u32);
    }
}
