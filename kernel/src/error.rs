use core::fmt::Display;

use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};

#[repr(C)]
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("Page Fault: No Free Page")]
    NoFreePage,

    #[error("Page Fault: Misaligned Virtual Address")]
    VirtualAddressMisaligned,

    #[error("Page Fault: Misaligned Size")]
    SizeMisaligned,

    #[error("Page Fault: Zero Size")]
    ZeroSize,

    #[error("Page Fault: Overflow")]
    AddressOverflow,

    #[error("Page Fault: Attempt to remap a valid page")]
    ValidPageRemap,

    #[error("Page Fault: Maximum number of processes exceeded")]
    NoUnusedProcess,

    #[error("Process Error: TrapFrame is None")]
    TrapFrameNone,

    #[error("Process Error: Failed to allocate kernel stack")]
    FailedToAllocateStack,

    #[error("Process Error: Failed to allocate trap frame")]
    FailedToAllocateTrapFrame,

    #[error("Process Error: Failed to allocate page table")]
    FailedToAllocatePageTable,

    #[error("Heap Error: Failed to allocate heap memory")]
    FailedToAllcateHeap,

    #[error("Page Fault: Page Table Entry is invalid")]
    PageTableEntryInvalid,

    #[error("Page Fault: Page Table Entry is not accessible in user mode")]
    PageTableEntryNotAccessibleInUserMode,

    #[error("Page Fault: Page corresponding to the page table entry is not allocated")]
    PageNotAllocated,

    #[error("Pipe Error: Write end of pipe is closed")]
    PipeWriterClosed,

    #[error("Pipe Error: Read end of pipe is closed")]
    PipeReaderClosed,

    #[error("Heap Error: Invalid heap size requested")]
    InvalidHeapSize,
}
