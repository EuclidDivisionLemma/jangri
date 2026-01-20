use core::fmt::Display;

use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};

use crate::fs::sfs::{DiskINode, MemoryINode};

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

    #[error("File System Error: No free i-node available")]
    NoFreeINode,

    #[error("File System Error: No free data block available")]
    NoFreeDataBlock,

    #[error("File System Error: No block on device")]
    NoBlockOnDevice,

    #[error("Coalescer Error: Intervals are not consecutive")]
    IntervalsNotConsecutive,

    #[error("File System Error: Byte offset is not zero when inode size is zero")]
    ByteOffsetNotZeroWhenInodeSizeIsZero,

    #[error("File System Error: Buffer size is greater than maximum file size")]
    FileSizeOverflow,

    #[error("File System Error: Attempt to read beyond end of file")]
    ReadBeyondEOF,

    #[error("File System Error: No such entry in directory: {name}")]
    NoSuchEntryInDirectory { name: String },

    #[error("File System Error: Inode number is zero")]
    InumZero,

    #[error("File System Error: Invalid path")]
    InvalidPath,

    #[error("File System Error: Not a directory: {name}")]
    NotADirectory { name: String },

    #[error("File System Error: File already exists: {path}")]
    FileAlreadyExists { path: String },

    #[error("File System Error: Attempt to perform an operation on a free inode: {inode:?}")]
    FreeInode { inode: DiskINode },

    #[error("File System Error: File does not exist: {path}")]
    FileDoesNotExist { path: String },

    #[error("Heap Error: Invalid heap size requested")]
    InvalidHeapSize,
}
