#![no_std]

use hal::interrupts::InterruptHandling;
use riscv::{ExceptionNumber, interrupt::Trap};
extern crate alloc;

use core::{
    arch::global_asm,
    sync::atomic::{AtomicBool, AtomicUsize},
};

use alloc::sync::Arc;
use hal::Hal;
use hal::error::Result;

use crate::vm::PageTableEntry;
pub use traps::handle_traps_from_supervisor_mode;

mod plic;
mod traps;
pub mod uart;
pub mod vm;

type Mutex<T> = sync::Mutex<T, PageTableEntry, Riscv>;

pub struct Riscv {
    pub allocate: Arc<dyn Fn(usize) -> Result<usize>>,
    pub deallocate: Arc<dyn Fn(usize, usize)>,
}

static NESTING_LEVEL: AtomicUsize = AtomicUsize::new(0);
static WERE_INTERRUPTS_ORIGINALLY_ENABLED: AtomicBool = AtomicBool::new(false);

global_asm!(
    r#"
    .section .text.entry
    .global entry
    entry:
        la sp, stack_top
        j main

    "#
);

impl Hal<vm::PageTableEntry> for Riscv {
    fn new(
        allocate: Arc<dyn Fn(usize) -> Result<usize>>,
        deallocate: Arc<dyn Fn(usize, usize)>,
    ) -> Self {
        plic::initialise();
        Self::initialise_traps();
        Self {
            allocate,
            deallocate,
        }
    }

    fn increase_nesting_level() {
        let _ = NESTING_LEVEL.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    }

    fn decrease_nesting_level() {
        let _ = NESTING_LEVEL.fetch_sub(1, core::sync::atomic::Ordering::SeqCst);
    }

    fn were_interrupts_originally_enabled() -> bool {
        WERE_INTERRUPTS_ORIGINALLY_ENABLED.load(core::sync::atomic::Ordering::Acquire)
    }

    fn set_original_interrupt_status(status: bool) {
        WERE_INTERRUPTS_ORIGINALLY_ENABLED.store(status, core::sync::atomic::Ordering::Release);
    }

    fn number_of_harts() -> usize {
        1
    }

    fn get_hart_id() -> usize {
        0
    }

    fn nesting_level() -> usize {
        NESTING_LEVEL.load(core::sync::atomic::Ordering::Acquire)
    }

    fn get_trampoline_offset() -> usize {
        todo!()
    }
}
