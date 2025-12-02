#![no_std]
#![no_main]
#![feature(core_float_math)]
#![allow(static_mut_refs)]

use core::arch::global_asm;

use crate::{
    constants::{END_OF_KERNEL_TEXT, KERNEL_END, KERNEL_PAGE_TABLE, KERNEL_START},
    syscall::stdout,
    vm::initialise_kernel_page_table,
};

mod allocator;
mod constants;
mod error;
mod panic;
mod syscall;
mod traps;
mod vm;

global_asm!(include_str!("entry.s"));

unsafe extern "C" {
    static mut kernel_end: u8;
    static mut end_of_kernel_text: u8;
    static mut kernel_start: u8;
}

fn intialise_constants() {
    unsafe { KERNEL_END = &kernel_end as *const u8 as usize }
    unsafe { END_OF_KERNEL_TEXT = &end_of_kernel_text as *const u8 as usize }
    unsafe { KERNEL_START = &kernel_start as *const u8 as usize }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    intialise_constants();
    if let Err(e) = initialise_kernel_page_table() {
        e.log(true);
    }
    unsafe {
        riscv::register::satp::set(
            riscv::register::satp::Mode::Sv48,
            0,
            KERNEL_PAGE_TABLE >> 12,
        );
    }
    stdout("Jangri v0.0.1\n");
    loop {}
}
