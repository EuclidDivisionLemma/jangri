use core::ptr::{read_volatile, write_volatile};

use crate::constants::{
    PLIC_S_MODE_CLAIM, PLIC_S_MODE_ENABLE, PLIC_S_MODE_THRESHOLD, UART_ID, UART_PRIORITY_ADDRESS,
};

pub fn initialise_plic() {
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
