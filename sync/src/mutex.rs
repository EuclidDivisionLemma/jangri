use hal::Hal;
use lock_api::GuardSend;

use core::{
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicUsize},
};

use crate::{pop, push};

pub struct Mutex<P: hal::vm::PageTableEntry, A: Hal<P>> {
    is_locked: AtomicBool,
    _a: PhantomData<A>,
    _p: PhantomData<P>,

    /// The hart current holding the lock is `hart_id + 1`. If no hart is holding the lock, the value
    /// shall be zero.
    holding_hart: AtomicUsize,
}

unsafe impl<P: hal::vm::PageTableEntry, A: Hal<P>> Sync for Mutex<P, A> {}
unsafe impl<P: hal::vm::PageTableEntry, A: Hal<P>> Send for Mutex<P, A> {}

impl<P: hal::vm::PageTableEntry, A: Hal<P>> Mutex<P, A> {
    fn is_current_hart_holding(&self) -> bool {
        if self.is_locked.load(core::sync::atomic::Ordering::Acquire)
            && self
                .holding_hart
                .load(core::sync::atomic::Ordering::Acquire)
                == A::get_hart_id() + 1
        {
            true
        } else {
            false
        }
    }
}

unsafe impl<P: hal::vm::PageTableEntry, A: Hal<P>> lock_api::RawMutex for Mutex<P, A> {
    const INIT: Self = Self {
        is_locked: AtomicBool::new(false),
        _a: PhantomData,
        _p: PhantomData,
        holding_hart: AtomicUsize::new(0),
    };

    type GuardMarker = GuardSend;

    fn lock(&self) {
        push::<P, A>();

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

        self.holding_hart
            .store(A::get_hart_id() + 1, core::sync::atomic::Ordering::Release);
    }

    fn try_lock(&self) -> bool {
        push::<P, A>();

        if let Err(_) = self.is_locked.compare_exchange(
            false,
            true,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        ) {
            return false;
        }

        self.holding_hart
            .store(A::get_hart_id() + 1, core::sync::atomic::Ordering::Release);

        true
    }

    unsafe fn unlock(&self) {
        self.holding_hart
            .store(0, core::sync::atomic::Ordering::Release);

        self.is_locked
            .store(false, core::sync::atomic::Ordering::Release);

        pop::<P, A>();
    }

    fn is_locked(&self) -> bool {
        let acquired_lock = self.try_lock();
        if acquired_lock {
            // Safety: The lock has been successfully acquired above.
            unsafe {
                self.unlock();
            }
        }
        !acquired_lock
    }
}
