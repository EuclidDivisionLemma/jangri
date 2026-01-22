#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "Virtual Memory Error: Attempt to remap a page at address {va} in page table at address {pt}"
    )]
    PageAlreadyMapped { va: usize, pt: usize },

    #[error(
        "Virtual Memory Error: Page starting at physical address {pa} not mapped in page table at address {pt}"
    )]
    NoSuchMapping { pa: usize, pt: usize },

    #[error("Virtual Memory Error: Virtual address {va} not mapped in page table at address {pt}")]
    NoSuchVirtualAddress { va: usize, pt: usize },

    #[error("Virtual Memory Error: Page table at address {0} is not valid")]
    PageTableInvalid(usize),

    #[error("Virtual Memory Error: Virtual Address ({0}) beyound range")]
    VirtualAddressOverflow(usize),

    #[error("Virtual Memory Error: Virtual Address ({0}) misaligned")]
    VirtualAddressMisaligned(usize),

    #[error("Virtual Memory Error: Size ({0}) misaligned")]
    SizeMisaligned(usize),

    #[error("Virtual Memory Error: Size is zero")]
    ZeroSize,
}
