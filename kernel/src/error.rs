use core::fmt::Display;

use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};

use crate::fs::sfs::{DiskINode, MemoryINode};

pub type Result<T> = core::result::Result<T, Error>;

#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    NoFreePage,
    VirtualAddressMisaligned,
    SizeMisaligned,
    ZeroSize,
    AddressOverflow,
    ValidPageRemap,
    NoUnusedProcess,
    TrapFrameNone,

    FailedToAllocateStack,
    FailedToAllocateTrapFrame,
    FailedToAllocatePageTable,
    FailedToAllcateHeap,

    PageTableEntryInvalid,
    PageTableEntryNotAccessibleInUserMode,

    PipeWriterClosed,
    PipeReaderClosed,

    NoFreeINode,
    NoFreeDataBlock,

    NoBlockOnDevice,

    IntervalsNotConsecutive,
    ByteOffsetNotZeroWhenInodeSizeIsZero,

    FileSizeOverflow,
    ReadBeyondEOF,
    NoSuchEntryInDirectory { name: String },
    InumZero,
    InvalidPath,
    NotADirectory { name: String },
    FileAlreadyExists { path: String },
    FreeInode { inode: DiskINode },
    FileDoesNotExist { path: String },
    InvalidHeapSize,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Error::NoFreePage => "Page Fault: No Free Page\n\n",
            Error::VirtualAddressMisaligned => "Page Fault: Misaligned Virtual Address\n\n",
            Error::SizeMisaligned => "Page Fault: Misaligned Size\n\n",
            Error::ZeroSize => "Page Fault: Zero Size\n\n",
            Error::AddressOverflow => "Page Fault: Overflow\n\n",

            Error::ValidPageRemap => "Page Fault: Attempt to remap a valid page\n\n",
            Error::NoUnusedProcess => "Page Fault: Maximum number of processes exceeded\n\n",
            Error::TrapFrameNone => "Process Error: TrapFrame is None\n\n",
            Error::FailedToAllocateStack => "Process Error: Failed to allocate kernel stack\n\n",
            Error::FailedToAllocateTrapFrame => "Process Error: Failed to allocate trap frame\n\n",
            Error::FailedToAllocatePageTable => "Process Error: Failed to allocate page table\n\n",
            Error::FailedToAllcateHeap => "Heap Error: Failed to allocate heap memory\n\n",
            Error::PageTableEntryInvalid => "Page Fault: Page Table Entry is invalid\n\n",
            Error::PageTableEntryNotAccessibleInUserMode => {
                "Page Fault: Page Table Entry is not accessible in user mode\n\n"
            }
            Error::PipeWriterClosed => "Pipe Error: Write end of pipe is closed\n\n",
            Error::NoFreeINode => "File System Error: No free i-node available\n\n",
            Error::NoFreeDataBlock => "File System Error: No free data block available\n\n",
            Error::NoBlockOnDevice => "File System Error: No block on device\n\n",
            Error::IntervalsNotConsecutive => "Coalescer Error: Intervals are not consecutive\n\n",
            Error::ByteOffsetNotZeroWhenInodeSizeIsZero => {
                "File System Error: Byte offset is not zero when inode size is zero\n\n"
            }
            Error::FileSizeOverflow => {
                "File System Error: Buffer size is greater than maximum file size\n\n"
            }
            Error::ReadBeyondEOF => "File System Error: Attempt to read beyond end of file\n\n",
            Error::InumZero => "File System Error: Inode number is zero\n\n",
            Error::InvalidPath => "File System Error: Invalid path\n\n",
            Error::NoSuchEntryInDirectory { name } => {
                &("File System Error: No such entry in directory: ".to_string() + name + "\n\n")
            }
            Error::NotADirectory { name } => {
                &("File System Error: Not a directory: ".to_string() + name + "\n\n")
            }
            Error::FileAlreadyExists { path } => {
                &("File System Error: File already exists: ".to_string() + path + "\n\n")
            }
            Error::FreeInode { inode } => {
                &("File System Error: Attempt to perform an operation a free inode: ".to_string()
                    + &format!("{:?}\n\n", inode))
            }
            Error::FileDoesNotExist { path } => {
                &("File System Error: File does not exist: ".to_string() + path + "\n\n")
            }
            Error::PipeReaderClosed => "Pipe Error: Read end of pipe is closed\n\n",
            Error::InvalidHeapSize => "Heap Error: Invalid heap size requested\n\n",
        };
        write!(f, "{}", text)
    }
}
