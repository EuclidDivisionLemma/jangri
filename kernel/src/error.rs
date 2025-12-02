use crate::syscall::stdout;

pub type Result<T> = core::result::Result<T, Error>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum Error {
    NoFreePage,
    VirtualAddressMisaligned,
    SizeMisaligned,
    ZeroSize,
    AddressOverflow,
    InvalidPageTableEntryAccess,
    ValidPageRemap,
    NoUnusedProcess,
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
            Error::InvalidPageTableEntryAccess => {
                text = "Page Fault: Page Table Entry (PTE) accessed is invalid\n\n";
            }
            Error::ValidPageRemap => text = "Page Fault: Attempt to remap a valid page\n\n",
            Error::NoUnusedProcess => text = "Page Fault: Maximum number of processes exceeded\n\n",
        }

        if panic == true {
            panic!("{text}");
        } else {
            stdout(text);
        }
    }
}
