#![no_std]
#![no_main]
#![feature(core_float_math)]
#![allow(static_mut_refs)]

use core::arch::global_asm;

use crate::constants::KERNEL_END;

mod allocator;
mod constants;
mod panic;

global_asm!(include_str!("entry.s"));

unsafe extern "C" {
    static mut kernel_end: u8;
}

fn intialise_constants() {
    unsafe { KERNEL_END = &kernel_end as *const u8 as usize }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    intialise_constants();

    loop {}
}
