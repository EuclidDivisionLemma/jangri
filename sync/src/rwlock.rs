use core::{marker::PhantomData, mem::ManuallyDrop};

use hal::{Hal, vm::PageTableEntry};
use lock_api::{GuardSend, RawRwLock};

pub struct RwLock<P: PageTableEntry, A: Hal<P>> {
    a: PhantomData<A>,
    p: PhantomData<P>,
    num_readers: crate::Mutex<usize, P, A>,
}

unsafe impl<P: PageTableEntry, A: Hal<P>> RawRwLock for RwLock<P, A> {
    const INIT: Self = RwLock {
        a: PhantomData,
        p: PhantomData,
        num_readers: crate::Mutex::new(0),
    };

    type GuardMarker = GuardSend;

    fn lock_shared(&self) {
        let mut num_readers = self.num_readers.lock();
        *num_readers += 1;
    }

    fn try_lock_shared(&self) -> bool {
        if let Some(mut num_readers) = self.num_readers.try_lock() {
            *num_readers += 1;
            return true;
        }

        return false;
    }

    unsafe fn unlock_shared(&self) {
        let mut num_readers = self.num_readers.lock();
        *num_readers -= 1;
    }

    fn lock_exclusive(&self) {
        loop {
            let mut num_readers = ManuallyDrop::new(self.num_readers.lock());

            if **num_readers == 0 {
                return;
            }

            unsafe {
                ManuallyDrop::drop(&mut num_readers);
            }
        }
    }

    fn try_lock_exclusive(&self) -> bool {
        let mut num_readers = ManuallyDrop::new(self.num_readers.lock());

        if **num_readers == 0 {
            return true;
        }

        unsafe {
            ManuallyDrop::drop(&mut num_readers);
        }
        return false;
    }

    unsafe fn unlock_exclusive(&self) {
        unsafe {
            self.num_readers.force_unlock();
        }
    }
}
