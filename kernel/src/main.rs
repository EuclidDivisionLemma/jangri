#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use core::arch::global_asm;

use alloc::format;

use crate::{
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_END, KERNEL_START, TRAMPOLINE_CODE_ADDRESS, TRAMPOLINE_OFFSET,
    },
    drivers::{
        Storage,
        ram_disk::RamDisk,
        uart::{console_write, initialise_uart},
    },
    file::{allocate_file, create_file, exists, traverse_path},
    fs::sfs::{self, DiskINode, flush_data_blocks, flush_inodes, read_inode},
    pipe::allocate_pipe,
    plic::initialise_plic,
    process::start_init,
    scheduler::schedule,
    syscall::stdout,
    traps::{initialise_traps, return_to_user_mode},
    vm::{align_to_page_size, enable_paging, initialise_kernel_page_table},
};

mod allocator;
mod constants;
mod drivers;
mod error;
mod file;
mod fs;
mod panic;
mod pipe;
mod plic;
mod process;
mod scheduler;
mod syscall;
mod traps;
mod vm;

extern crate alloc;

global_asm!(
    r#"
    .section .text.entry
    .global entry
    entry:
        la sp, stack_top
        j main

    "#
);

pub const INIT: &[u8] = include_bytes!("../../userspace/init.elf");

unsafe extern "C" {
    static kernel_end: u8;
    static end_of_kernel_text: u8;
    static kernel_start: u8;
    static trampoline_code_address: u8;
}

pub const DEVICE: RamDisk = RamDisk;

fn intialise_constants() {
    unsafe {
        KERNEL_END = align_to_page_size(&kernel_end as *const u8 as usize);
        END_OF_KERNEL_TEXT = align_to_page_size(&end_of_kernel_text as *const u8 as usize);
        KERNEL_START = align_to_page_size(&kernel_start as *const u8 as usize);
        TRAMPOLINE_CODE_ADDRESS = &trampoline_code_address as *const u8 as usize;
        TRAMPOLINE_OFFSET = return_to_user_mode as usize - TRAMPOLINE_CODE_ADDRESS;
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    intialise_constants();

    initialise_kernel_page_table().unwrap();

    enable_paging();

    let ram_disk = RamDisk;
    ram_disk.initialise();

    initialise_traps();

    initialise_plic();
    initialise_uart();
    fs::initialise();

    console_write("\x1b[2J\x1b[HJangri v0.0.1\n");

    start_init();

    schedule();
}
