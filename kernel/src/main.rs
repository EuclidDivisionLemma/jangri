#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use hal::interrupts::InterruptHandling;

use crate::{
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_END, KERNEL_START, TRAMPOLINE_CODE_ADDRESS, TRAMPOLINE_OFFSET,
    },
    global_state::GlobalState,
    process::start_process,
    scheduler::schedule,
    syscall::stdout,
    vm::{align_to_page_size, enable_paging, initialise_kernel_page_table},
};

mod allocator;
mod constants;
mod global_state;
mod panic;
mod pipe;
mod process;
mod scheduler;
mod syscall;
mod traps;
mod vm;

extern crate alloc;

pub const PROCESS1: &[u8] = include_bytes!("../../userspace/process1.elf");
pub const PROCESS2: &[u8] = include_bytes!("../../userspace/process2.elf");

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
    intialise_constants();

    let state = GlobalState::initialise();

    initialise_kernel_page_table(&state).unwrap();

    enable_paging();

    traps::initialise();
    uart::initialise();

    stdout("\x1b[2J\x1b[HJangri v0.0.1\n");

    start_process(state, PROCESS1, "process1");
    start_process(state, PROCESS2, "process2");

    schedule(state);
}
