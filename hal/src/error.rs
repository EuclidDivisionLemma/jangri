use core::fmt::Debug;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    PageAlreadyMapped { va: usize, pt: usize },
    NoSuchVirtualAddress { va: usize, pt: usize },
    VirtualAddressOverflow(usize),
    VirtualAddressMisaligned(usize),
    SizeMisaligned(usize),
    ZeroSize,
    MemoryNotAvailable,
    NotASyscall,
    ELFError,
    ExplicitPanic,
    BeyondAllocationLimit,
}
