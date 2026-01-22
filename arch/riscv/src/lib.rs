#![no_std]

extern crate alloc;

use core::sync::atomic::{AtomicBool, AtomicUsize};

use alloc::sync::Arc;
use anyhow::Result;
use hal::{Hal, interrupts::InterruptHandling};

pub mod vm;

#[cfg(test)]
mod tests;

pub struct Riscv {
    pub allocate: Arc<dyn Fn(usize) -> Result<usize>>,
    pub deallocate: Arc<dyn Fn(usize, usize)>,
}

static NESTING_LEVEL: AtomicUsize = AtomicUsize::new(0);
static WERE_INTERRUPTS_ORIGINALLY_ENABLED: AtomicBool = AtomicBool::new(false);

impl InterruptHandling for Riscv {
    unsafe fn enable_interrupts() {
        unsafe {
            riscv::interrupt::supervisor::enable();
        }
    }

    fn disable_interrupts() {
        riscv::interrupt::supervisor::disable();
    }

    fn set_next_timer_interrupt(time: usize) {
        todo!()
    }

    fn are_interrupts_enabled() -> bool {
        riscv::register::sstatus::read().sie()
    }
}

impl Hal<vm::PageTableEntry> for Riscv {
    fn new(
        allocate: Arc<dyn Fn(usize) -> Result<usize>>,
        deallocate: Arc<dyn Fn(usize, usize)>,
    ) -> Self {
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
}
