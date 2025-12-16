use crate::drivers::uart::console_write;

pub type Result<T> = core::result::Result<T, Error>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
}

impl Error {
    pub fn log(&self, panic: bool) {
        let text: &str;
        match self {
            Error::NoFreePage => text = "Page Fault: No Free Page\n\n",
            Error::VirtualAddressMisaligned => text = "Page Fault: Misaligned Virtual Address\n\n",
            Error::SizeMisaligned => text = "Page Fault: Misaligned Size\n\n",
            Error::ZeroSize => text = "Page Fault: Zero Size\n\n",
            Error::AddressOverflow => text = "Page Fault: Overflow\n\n",

            Error::ValidPageRemap => text = "Page Fault: Attempt to remap a valid page\n\n",
            Error::NoUnusedProcess => text = "Page Fault: Maximum number of processes exceeded\n\n",
            Error::TrapFrameNone => text = "Process Error: TrapFrame is None\n\n",
            Error::FailedToAllocateStack => {
                text = "Process Error: Failed to allocate kernel stack\n\n"
            }
            Error::FailedToAllocateTrapFrame => {
                text = "Process Error: Failed to allocate trap frame\n\n"
            }
            Error::FailedToAllocatePageTable => {
                text = "Process Error: Failed to allocate page table\n\n"
            }
            Error::FailedToAllcateHeap => text = "Heap Error: Failed to allocate heap memory\n\n",
            Error::PageTableEntryInvalid => text = "Page Fault: Page Table Entry is invalid\n\n",
            Error::PageTableEntryNotAccessibleInUserMode => {
                text = "Page Fault: Page Table Entry is not accessible in user mode\n\n"
            }
        }

        if panic == true {
            panic!("{text}");
        } else {
            console_write(text);
        }
    }
}
