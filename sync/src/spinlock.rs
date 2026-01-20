use alloc::vec::Vec;

use crate::{HARTS, Hart, pop};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicUsize},
};

use crate::{Lock, MutexGuard, push};

pub struct Mutex<T> {
    is_locked: AtomicBool,
    data: UnsafeCell<T>,
    holding_hart: AtomicUsize,
    get_current_hart: fn() -> usize,
    interrupts_enable: unsafe fn(),
    interrupts_disable: fn(),
    are_interrupts_enabled: fn() -> bool,
}

impl<T> Mutex<T> {
    pub fn new(
        data: T,
        num_harts: usize,
        get_current_hart: fn() -> usize,
        interrupts_enable: unsafe fn(),
        interrupts_disable: fn(),
        are_interrupts_enabled: fn() -> bool,
    ) -> Self {
        interrupts_disable();

        unsafe {
            if HARTS.is_empty() {
                HARTS = Vec::from_fn(num_harts, |_| Hart {
                    were_interrupts_originally_enabled: false,
                    nesting_level: 0,
                });
            }
        }

        unsafe {
            interrupts_enable();
        }

        Mutex {
            is_locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            holding_hart: AtomicUsize::new(0),
            get_current_hart,
            interrupts_enable,
            interrupts_disable,
            are_interrupts_enabled,
        }
    }
}

impl<T> Lock<T> for Mutex<T> {
    fn is_current_hart_holding(&self) -> bool {
        if self.is_locked.load(core::sync::atomic::Ordering::Acquire)
            && self
                .holding_hart
                .load(core::sync::atomic::Ordering::Acquire)
                == (self.get_current_hart)() + 1
        {
            true
        } else {
            false
        }
    }

    fn lock<'a>(&'a self) -> MutexGuard<'a, T> {
        push(
            self.get_current_hart,
            self.interrupts_disable,
            self.are_interrupts_enabled,
        );

        if self.is_current_hart_holding() {
            panic!(
                "DEADLOCK: Hart {} called lock twice.\n",
                self.holding_hart
                    .load(core::sync::atomic::Ordering::Acquire)
                    - 1,
            );
        }

        while let Err(_) = self.is_locked.compare_exchange(
            false,
            true,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        ) {
            core::hint::spin_loop();
        }

        self.holding_hart.store(
            (self.get_current_hart)() + 1,
            core::sync::atomic::Ordering::Release,
        );

        MutexGuard { lock: self }
    }

    fn try_lock<'a>(&'a self) -> Option<MutexGuard<'a, T>> {
        push(
            self.get_current_hart,
            self.interrupts_disable,
            self.are_interrupts_enabled,
        );

        if self.is_current_hart_holding() {
            panic!(
                "DEADLOCK: Hart {} called lock twice.\n",
                self.holding_hart
                    .load(core::sync::atomic::Ordering::Acquire)
                    - 1,
            );
        }

        if let Err(_) = self.is_locked.compare_exchange(
            false,
            true,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        ) {
            return None;
        }

        self.holding_hart.store(
            (self.get_current_hart)() + 1,
            core::sync::atomic::Ordering::Release,
        );

        Some(MutexGuard { lock: self })
    }

    fn set(&self, data: T) {
        let guard = self.lock();
        // old gets dropped as its scope ends
        unsafe { guard.lock.data().replace(data) };
    }

    fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    fn data(&self) -> &UnsafeCell<T> {
        &self.data
    }

    unsafe fn unlock(&self) {
        self.holding_hart
            .store(0, core::sync::atomic::Ordering::Release);

        self.is_locked
            .store(false, core::sync::atomic::Ordering::Release);

        pop(self.get_current_hart, self.interrupts_enable);
    }
}

unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Send for Mutex<T> {}
