#![no_std]
#![feature(stmt_expr_attributes)]
#![feature(associated_type_defaults)]

extern crate alloc;

use core::ops::Fn;
use core::ops::{BitAndAssign, BitOrAssign, Not};

use alloc::sync::Arc;
use anyhow::Result;

use crate::{
    interrupts::InterruptHandling,
    vm::{PageTableEntry, VirtualMemory},
};

pub mod constants;
mod cpu;
pub mod error;
pub mod interrupts;
pub mod vm;

pub trait Hal<T: PageTableEntry>: VirtualMemory<T> + InterruptHandling {
    fn new(
        allocate: Arc<dyn Fn(usize) -> Result<usize>>,
        deallocate: Arc<dyn Fn(usize, usize)>,
    ) -> Self;
    fn increase_nesting_level();
    fn decrease_nesting_level();
    fn were_interrupts_originally_enabled() -> bool;
    fn set_original_interrupt_status(status: bool);
    fn number_of_harts() -> usize;
    fn get_hart_id() -> usize;
    fn nesting_level() -> usize;
    fn get_trampoline_offset() -> usize;
}

#[inline(always)]
pub fn set_bit<T: BitOrAssign>(src: T, mut target: T) -> T {
    target |= src;
    target
}

#[inline(always)]
pub fn clear_bit<T: BitAndAssign + Not<Output = T>>(src: T, mut target: T) -> T {
    target &= !src;
    target
}
