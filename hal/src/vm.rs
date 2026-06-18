use core::fmt::Debug;

use crate::error::{Error, Result};
use alloc::sync::Arc;

use crate::constants::{MAX_VA, NUMBER_OF_LEVELS, PAGE_SIZE, TRAPFRAME};
use crate::vm::constants::NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE;

pub trait VirtualMemory<T: PageTableEntry> {
    fn map(
        &self,
        page_table: *mut PageTable<T>,
        va: usize,
        pa: usize,
        size: usize,
        read: bool,
        write: bool,
        execute: bool,
        user: bool,
    ) -> Result<()>;
    fn unmap(
        &self,
        page_table: *mut PageTable<T>,
        va: usize,
        num_pages: usize,
        deallocate: bool,
    ) -> Result<()>;
    fn va2pa(&self, page_table: *mut PageTable<T>, va: usize) -> Result<usize>;
    fn clean_up_page_table(&self, page_table: *mut PageTable<T>) -> Result<()>;
    fn enable_paging(page_table: usize);
}

pub trait PageTableEntry: Copy + Debug {
    fn set_user_mode(&mut self) -> &mut Self;
    fn set_read(&mut self) -> &mut Self;
    fn set_write(&mut self) -> &mut Self;
    fn set_execute(&mut self) -> &mut Self;
    fn set_valid(&mut self) -> &mut Self;
    fn get_physical_address(&self) -> usize;
    fn set_physical_address(&mut self, pa: usize) -> &mut Self;
    fn is_valid(&self) -> bool;
    fn is_leaf_pte(&self) -> bool;
    fn clear_bits(&mut self) -> &mut Self;
    fn readable(&self) -> bool;
    fn writeable(&self) -> bool;
    fn executable(&self) -> bool;
    fn user_mode_accessible(&self) -> bool;
    fn supervisor_accessible(&self) -> bool;
}

#[repr(transparent)]
pub struct PageTable<T: PageTableEntry> {
    entries: [T; NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE],
}

impl<T: PageTableEntry> PageTable<T> {
    pub fn get_entry(&mut self, index: usize) -> *mut T {
        &raw mut self.entries[index]
    }

    pub fn level_to_index(&self, level: usize, va: usize) -> usize {
        #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
        return va >> (12 + (level * 9)) & 0b111111111;
        unreachable!()
    }

    /// Returns the address of the page table entry corresponding to the given virtual address.
    pub fn get_page_table_entry_address(
        &mut self,
        allocate: Arc<dyn Fn(usize) -> Result<usize>>,
        va: usize,
        should_allocate: bool,
    ) -> Result<*mut T> {
        if va > MAX_VA {
            return Err(Error::VirtualAddressOverflow(va));
        }

        let mut page_table: *mut Self = &raw mut *self;
        let mut page_table_entry: *mut T;

        for level in (1..NUMBER_OF_LEVELS).rev() {
            unsafe {
                page_table_entry = (*page_table).get_entry((*page_table).level_to_index(level, va));
                // address of page table entry

                if (*page_table_entry).is_valid()
                // if page table entry is valid
                {
                    page_table = (*page_table_entry).get_physical_address() as *mut Self;
                } else {
                    if should_allocate {
                        match allocate(PAGE_SIZE) {
                            Ok(v) => page_table = v as *mut Self,
                            Err(e) => return Err(e.into()),
                        }

                        (*page_table_entry)
                            .set_physical_address(page_table as usize)
                            .set_valid();
                    } else {
                        return Err(Error::NoSuchVirtualAddress {
                            va: va,
                            pt: page_table as usize,
                        });
                    }
                }
            }
        }

        let final_pte = unsafe { (*page_table).get_entry((*page_table).level_to_index(0, va)) };
        Ok(final_pte)
    }
}

#[cfg(target_arch = "riscv64")]
pub(crate) mod constants {
    pub const NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE: usize = 512;
    pub const MAX_VA: usize = 0xffffffffffffffff;
    pub const NUMBER_OF_LEVELS: usize = 4;
    pub const PAGE_SIZE: usize = 0x1000;
}

#[cfg(not(target_arch = "riscv64"))]
pub(crate) mod constants {
    pub const NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE: usize = 0;
    pub const MAX_VA: usize = 0;
    pub const NUMBER_OF_LEVELS: usize = 0;
    pub const PAGE_SIZE: usize = 1;
}

pub fn align_to_page_size(size: usize) -> usize {
    let offset = size % PAGE_SIZE;
    let base = size / PAGE_SIZE;

    if offset == 0 {
        base * PAGE_SIZE
    } else {
        (base + 1) * PAGE_SIZE
    }
}
