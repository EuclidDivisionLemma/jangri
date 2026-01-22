use core::sync::atomic::{AtomicBool, AtomicUsize};

pub struct Hart {
    nesting_level: AtomicUsize,
    were_interrupts_originally_enabled: AtomicBool,
    hart_id: usize,
}
