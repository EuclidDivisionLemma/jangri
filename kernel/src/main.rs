#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use core::arch::asm;

use alloc::format;
use hal::{interrupts::InterruptHandling, vm::align_to_page_size};
use spin::once::Once;

use crate::{
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_END, KERNEL_PAGE_TABLE, KERNEL_START, TRAMPOLINE_CODE_ADDRESS,
        TRAMPOLINE_OFFSET,
    },
    device_tree::NUM_CPUS,
    global_state::GlobalState,
    process::assign_process,
    scheduler::schedule,
    vm::{enable_paging, initialise_kernel_page_table},
};
use hal::Hal;

mod allocator;
mod constants;
mod device_tree;
mod global_state;
mod panic;
mod process;
mod scheduler;
mod syscall;
mod traps;
mod vm;

extern crate alloc;

#[cfg(target_arch = "riscv64")]
pub type ARCH = riscv_arch::Riscv;

#[cfg(target_arch = "riscv64")]
#[allow(non_camel_case_types)]
pub type PAGE_TABLE_ENTRY = riscv_arch::vm::PageTableEntry;

pub type Mutex<T> = sync::Mutex<T, PAGE_TABLE_ENTRY, ARCH>;
pub type RwLock<T> = sync::RwLock<T, PAGE_TABLE_ENTRY, ARCH>;
pub type TrapFrame = <ARCH as InterruptHandling>::TRAPFRAME;
pub type RawMutex = sync::RawMutex<PAGE_TABLE_ENTRY, ARCH>;

#[cfg(target_arch = "riscv64")]
use riscv_arch::uart;

unsafe extern "C" {
    static kernel_end: u8;
    static end_of_kernel_text: u8;
    static kernel_start: u8;
    static trampoline_code_address: u8;

    fn return_to_user_mode();
    fn entry_sec();
}

pub static SH: &'static [u8] = include_bytes!("../../sh.bin");

#[macro_export]
macro_rules! print {
    ($($x: expr,)*) => {
        uart::console_write(&format!($($x,)*));
    };
    ($x: expr) => {
        uart::console_write($x);
    };
}

#[macro_export]
macro_rules! println {
    ($($x: expr),*) => {
        print!($($x,)*);
        uart::console_write("\n");
    };
    ($x: expr) => {
        uart::console_write($x);
    };
}

fn intialise_constants() {
    unsafe {
        KERNEL_END = align_to_page_size(&kernel_end as *const u8 as usize);
        END_OF_KERNEL_TEXT = align_to_page_size(&end_of_kernel_text as *const u8 as usize);
        KERNEL_START = align_to_page_size(&kernel_start as *const u8 as usize);
        TRAMPOLINE_CODE_ADDRESS = &trampoline_code_address as *const u8 as usize;
        TRAMPOLINE_OFFSET =
            return_to_user_mode as unsafe extern "C" fn() as usize - TRAMPOLINE_CODE_ADDRESS;
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    device_tree::initialise();

    println!("\x1b[2J\x1b[HJangri v0.0.1\n");

    device_tree::find_cpus();
    println!("Number of CPUs: {}\n", NUM_CPUS.get().unwrap());

    println!("[1 of 5] Initialising Constants");
    intialise_constants();

    println!("[2 of 5] Setting up the Kernel");
    let state = GlobalState::initialise();

    println!("[3 of 5] Setting up Virtual Memory");
    initialise_kernel_page_table(&state).unwrap();
    enable_paging();

    println!("[4 of 5] Setting up Console");
    traps::initialise();
    uart::initialise();

    println!("[5 of 5] Starting shell\n");
    assign_process(state, "sh", SH.to_vec()).unwrap();

    let pa = state
        .va2pa(unsafe { KERNEL_PAGE_TABLE }, entry_sec as usize)
        .unwrap();
    ARCH::wake_up_other_cores(device_tree::cpu_mask(), pa);

    schedule(state);
}

#[unsafe(no_mangle)]
fn main_secondary() {
    loop {}
}
