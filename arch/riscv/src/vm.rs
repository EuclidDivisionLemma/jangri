use core::arch::asm;

use alloc::sync::Arc;
use anyhow::bail;

mod pte;

use hal::{
    constants::{NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE, PAGE_SIZE},
    error::Error,
    vm::{PageTable, PageTableEntry as PageTableEntryTrait, VirtualMemory},
};

pub use crate::{Riscv, vm::pte::PageTableEntry};

impl VirtualMemory<PageTableEntry> for Riscv {
    fn map(
        &self,
        page_table: *mut hal::vm::PageTable<PageTableEntry>,
        mut va: usize,
        mut pa: usize,
        size: usize,
        read: bool,
        write: bool,
        execute: bool,
        user: bool,
    ) -> anyhow::Result<()> {
        let mut page_table_entry: *mut PageTableEntry;

        if va % PAGE_SIZE != 0 {
            bail!(Error::VirtualAddressMisaligned(va));
        }

        if size % PAGE_SIZE != 0 {
            bail!(Error::SizeMisaligned(size));
        }

        if size == 0 {
            bail!(Error::ZeroSize);
        }

        let number_of_pages = size / PAGE_SIZE;

        for _ in 0..number_of_pages {
            match unsafe {
                (*page_table).get_page_table_entry_address(self.allocate.clone(), va, true)
            } {
                Ok(v) => page_table_entry = v,
                Err(e) => bail!(e),
            }

            unsafe {
                if (*page_table_entry).is_valid() {
                    bail!(Error::PageAlreadyMapped {
                        va,
                        pt: (&raw const page_table) as usize
                    });
                }

                (*page_table_entry).set_physical_address(pa).set_valid();

                if read {
                    (*page_table_entry).set_read();
                }

                if write {
                    (*page_table_entry).set_write();
                }

                if execute {
                    (*page_table_entry).set_execute();
                }

                if user {
                    (*page_table_entry).set_user_mode();
                }
            }

            va = va.wrapping_add(PAGE_SIZE);

            pa += PAGE_SIZE;
        }

        Ok(())
    }

    fn unmap(
        &self,
        page_table: *mut hal::vm::PageTable<PageTableEntry>,
        va: usize,
        num_pages: usize,
        deallocate: bool,
    ) -> anyhow::Result<()> {
        for page_va in (va..va + num_pages * PAGE_SIZE).step_by(PAGE_SIZE) {
            if let Ok(pte) = unsafe {
                (*page_table).get_page_table_entry_address(self.allocate.clone(), page_va, false)
            } {
                if unsafe { (*pte).is_valid() } {
                    continue;
                }

                if deallocate {
                    (self.deallocate)(unsafe { (*pte).get_physical_address() }, PAGE_SIZE);
                }

                unsafe { (*pte).clear_bits() };
            }
        }

        Ok(())
    }

    fn va2pa(
        &self,
        page_table: *mut hal::vm::PageTable<PageTableEntry>,
        va: usize,
    ) -> anyhow::Result<usize> {
        let aligned_va = (va / PAGE_SIZE) * PAGE_SIZE;
        let offset = va % PAGE_SIZE;

        let page_table_entry = unsafe {
            (*page_table).get_page_table_entry_address(self.allocate.clone(), aligned_va, false)
        }?;

        if unsafe { !(*page_table_entry).is_valid() } {
            panic!();
        }

        Ok(unsafe { (*page_table_entry).get_physical_address() } + offset)
    }

    fn clean_up_page_table(
        &self,
        page_table: *mut hal::vm::PageTable<PageTableEntry>,
    ) -> anyhow::Result<()> {
        for i in 0..NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE {
            let pte = unsafe { (*page_table).get_entry(i) };
            let page_table =
                unsafe { (*pte).get_physical_address() as *mut PageTable<PageTableEntry> };

            recursive_clean(self.deallocate.clone(), page_table);
        }

        (self.deallocate)(page_table as usize, PAGE_SIZE);
        Ok(())
    }

    fn enable_paging(page_table: usize) {
        unsafe {
            asm!("sfence.vma");
            riscv::register::satp::set(riscv::register::satp::Mode::Sv48, 0, page_table >> 12);
            asm!("sfence.vma");
        }
    }
}

fn recursive_clean(
    deallocate: Arc<dyn Fn(usize, usize)>,
    page_table: *mut PageTable<PageTableEntry>,
) {
    unsafe {
        for i in 0..NUMBER_OF_PAGE_TABLE_ENTRIES_PER_PAGE {
            let pte = (*page_table).get_entry(i);

            if (*pte).is_valid() {
                if !(*pte).readable() && !(*pte).writeable() && !(*pte).executable() {
                    recursive_clean(
                        deallocate.clone(),
                        (*pte).get_physical_address() as *mut PageTable<PageTableEntry>,
                    );
                }
                let pa = (*pte).get_physical_address();
                deallocate(pa, PAGE_SIZE);
                (*pte).clear_bits();
            }
        }
    }
}
