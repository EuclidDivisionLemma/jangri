#![no_std]
#![allow(static_mut_refs)]

extern crate alloc;

mod mutex;
mod rwlock;

use hal::{Hal, vm::PageTableEntry};

pub type RawMutex<P, A> = mutex::Mutex<P, A>;
pub type RawRwLock<P, A> = rwlock::RwLock<P, A>;
pub type Mutex<T, P, A> = lock_api::Mutex<RawMutex<P, A>, T>;
pub type RwLock<T, P, A> = lock_api::RwLock<RawRwLock<P, A>, T>;

fn push<P: PageTableEntry, A: Hal<P>>() {
    let were_interrupts_originally_enabled = A::are_interrupts_enabled();

    A::disable_interrupts();

    if A::nesting_level() == 0 {
        A::set_original_interrupt_status(were_interrupts_originally_enabled);
    }

    A::increase_nesting_level();
}

pub fn pop<P: PageTableEntry, A: Hal<P>>() {
    unsafe {
        A::decrease_nesting_level();

        if A::nesting_level() == 0 && A::were_interrupts_originally_enabled() {
            A::enable_interrupts();
        }
    }
}
