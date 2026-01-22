use talc::{ClaimOnOom, Span, Talc, Talck};

use crate::{ARCH, PAGE_TABLE_ENTRY, constants::KERNEL_HEAP_SIZE};

pub static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
pub static HEAP_ALLOCATOR: Talck<sync::RawMutex<PAGE_TABLE_ENTRY, ARCH>, ClaimOnOom> =
    Talck::new(Talc::new(unsafe {
        ClaimOnOom::new(Span::from_array(&raw mut HEAP))
    }));
