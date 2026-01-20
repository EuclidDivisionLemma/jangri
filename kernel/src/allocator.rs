use sync::Mutex;
use talc::{ClaimOnOom, Span, Talc, Talck};

use crate::constants::KERNEL_HEAP_SIZE;

pub static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
pub static HEAP_ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talck::new(Talc::new(unsafe {
    ClaimOnOom::new(Span::from_array(&raw mut HEAP))
}));
