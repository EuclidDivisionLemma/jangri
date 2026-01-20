#![no_std]
#![feature(unsafe_cell_access)]
#![feature(vec_from_fn)]
#![allow(static_mut_refs)]

extern crate alloc;

mod spinlock;

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

use alloc::vec::Vec;
pub use spinlock::Mutex;

// pub fn push() {
//     unsafe {
//         let were_interrupts_originally_enabled = are_interrupts_enabled();

//         riscv::interrupt::supervisor::disable();

//         let hart = get_hart();

//         if *hart.nesting_level.get() == 0 {
//             *hart.were_interrupts_originally_enabled.get() = were_interrupts_originally_enabled;
//         }

//         *hart.nesting_level.get() += 1;
//     }
// }

// pub fn pop() {
//     unsafe {
//         let hart = get_hart();

//         *hart.nesting_level.get() -= 1;

//         if *hart.nesting_level.as_ref_unchecked() == 0
//             && *hart.were_interrupts_originally_enabled.as_ref_unchecked()
//         {
//             riscv::interrupt::supervisor::enable();
//         }
//     }
// }
struct Hart {
    were_interrupts_originally_enabled: bool,
    nesting_level: usize,
}

static mut HARTS: Vec<Hart> = Vec::new();

pub trait Lock<T> {
    fn is_current_hart_holding(&self) -> bool;
    fn lock<'a>(&'a self) -> MutexGuard<'a, T>;
    fn try_lock<'a>(&'a self) -> Option<MutexGuard<'a, T>>;
    fn set(&self, data: T);
    fn get_mut(&mut self) -> &mut T;
    fn data(&self) -> &UnsafeCell<T>;
    unsafe fn unlock(&self);
}

pub struct MutexGuard<'a, T> {
    lock: &'a dyn Lock<T>,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data().get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data().get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { self.lock.unlock() }
    }
}

fn push(
    get_current_hart: fn() -> usize,
    interrupts_disable: fn(),
    are_interrupts_enabled: fn() -> bool,
) {
    let were_interrupts_originally_enabled = are_interrupts_enabled();

    interrupts_disable();

    let hart = get_current_hart();

    unsafe {
        if HARTS[hart].nesting_level == 0 {
            HARTS[hart].were_interrupts_originally_enabled = were_interrupts_originally_enabled;
        }

        HARTS[hart].nesting_level += 1;
    }
}

pub fn pop(get_current_hart: fn() -> usize, interrupts_enable: unsafe fn()) {
    let hart = get_current_hart();

    unsafe {
        HARTS[hart].nesting_level -= 1;

        if HARTS[hart].nesting_level == 0 && HARTS[hart].were_interrupts_originally_enabled {
            interrupts_enable();
        }
    }
}
