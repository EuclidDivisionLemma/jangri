use alloc::format;
use anyhow::{Result, bail};
use sync::Lock;

use crate::{
    constants::{
        END_OF_KERNEL_TEXT, EXECUTE_ONLY, KERNEL_PAGE_TABLE, KERNEL_START, MAX_VA, MAXIMUM_PROCESS,
        PAGE_SIZE, PLIC, PLIC_SIZE, RAM_STOP, READ_EXECUTE, READ_ONLY, READ_WRITE, STACK_PAGES,
        STACK_START, TRAMPOLINE, TRAMPOLINE_CODE_ADDRESS, TRAPFRAME, UART0, USER_MODE, VALID_BIT,
        VIRTIO_MMIO_DISK, VIRTIO_MMIO_DISK_SIZE, WRITE_ONLY,
    },
    error::{self, Error},
    global_state::GlobalState,
    syscall::stdout,
    traps::TrapFrame,
};
use core::ptr::write_volatile;
use core::{
    arch::asm,
    ptr::{self, read_volatile},
};

#[inline(always)]
pub fn extract_index_into_level(level: usize, virtual_address: usize) -> usize {
    virtual_address >> (12 + (level * 9)) & 0b111111111
}

#[inline(always)]
pub fn page_table_entry_to_physical_address(page_table_entry: usize) -> usize {
    (page_table_entry >> 10) << 12
}

#[inline(always)]
pub fn physical_address_to_page_table_entry(physical_address: usize) -> usize {
    (physical_address >> 12) << 10
}

pub fn enable_paging() {
    unsafe {
        riscv::register::satp::set(
            riscv::register::satp::Mode::Sv48,
            0,
            KERNEL_PAGE_TABLE >> 12,
        );
        asm!("sfence.vma");
    }
}

pub fn align_to_page_size(size: usize) -> usize {
    (size + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE
}

pub fn initialise_kernel_page_table(state: &GlobalState) -> Result<()> {
    unsafe {
        KERNEL_PAGE_TABLE = state.allocate(PAGE_SIZE).unwrap();

        map_kernel_stack(state);

        // map UART registers
        map(
            state,
            KERNEL_PAGE_TABLE,
            UART0,
            UART0,
            PAGE_SIZE,
            READ_WRITE,
        )?;

        // map VIRIO MMIO Disk Registers
        map(
            state,
            KERNEL_PAGE_TABLE,
            VIRTIO_MMIO_DISK,
            VIRTIO_MMIO_DISK,
            VIRTIO_MMIO_DISK_SIZE,
            READ_WRITE,
        )?;

        // map PLIC Registers
        map(state, KERNEL_PAGE_TABLE, PLIC, PLIC, PLIC_SIZE, READ_WRITE)?;

        // map kernel code
        map(
            state,
            KERNEL_PAGE_TABLE,
            KERNEL_START,
            KERNEL_START,
            END_OF_KERNEL_TEXT - KERNEL_START,
            READ_EXECUTE,
        )?;

        // map kernel data and RAM
        map(
            state,
            KERNEL_PAGE_TABLE,
            END_OF_KERNEL_TEXT,
            END_OF_KERNEL_TEXT,
            RAM_STOP - END_OF_KERNEL_TEXT,
            READ_WRITE,
        )?;

        // The trampoline page is mapped at the highest virtual address
        // in both user and kernel page tables so that we can jump to
        // it in either mode.
        map_trampoline(
            state,
            KERNEL_PAGE_TABLE,
            TRAMPOLINE,
            TRAMPOLINE_CODE_ADDRESS,
            PAGE_SIZE,
            READ_EXECUTE,
        )?;
    }

    Ok(())
}

pub fn map(
    state: &GlobalState,
    page_table: usize,
    virtual_address: usize,
    physical_address: usize,
    size: usize,
    permissions: usize,
) -> Result<()> {
    map_pages(
        state,
        page_table,
        virtual_address,
        physical_address,
        size,
        permissions,
        false,
    )?;

    Ok(())
}

pub fn map_trampoline(
    state: &GlobalState,
    page_table: usize,
    virtual_address: usize,
    physical_address: usize,
    size: usize,
    permissions: usize,
) -> Result<()> {
    map_pages(
        state,
        page_table,
        virtual_address,
        physical_address,
        size,
        permissions,
        true,
    )?;

    Ok(())
}

pub fn map_pages(
    state: &GlobalState,
    page_table: usize,
    mut virtual_address: usize,
    mut physical_address: usize,
    size: usize,
    permissions: usize,
    trampoline: bool,
) -> Result<()> {
    let mut page_table_entry_address: usize;

    if virtual_address % PAGE_SIZE != 0 {
        bail!(error::Error::VirtualAddressMisaligned);
    }

    if size % PAGE_SIZE != 0 {
        bail!(error::Error::SizeMisaligned);
    }

    if size == 0 {
        bail!(error::Error::ZeroSize);
    }

    let number_of_pages = size / PAGE_SIZE;

    for _ in 0..number_of_pages {
        match get_page_table_entry_address(state, page_table, virtual_address, true) {
            Ok(v) => page_table_entry_address = v,
            Err(e) => bail!(e),
        }

        unsafe {
            if read_volatile(page_table_entry_address as *const usize) & VALID_BIT == 0b1 {
                bail!(error::Error::ValidPageRemap);
            }

            write_volatile(
                page_table_entry_address as *mut usize,
                physical_address_to_page_table_entry(physical_address) | permissions | VALID_BIT,
            );
        }

        // When mapping trampoline, VA overflows
        virtual_address = if trampoline {
            virtual_address.wrapping_add(PAGE_SIZE)
        } else {
            virtual_address + PAGE_SIZE
        };

        physical_address += PAGE_SIZE;
    }

    Ok(())
}

/// Returns the address of the page table entry corresponding to the given virtual address.
pub fn get_page_table_entry_address(
    state: &GlobalState,
    mut page_table: usize,
    virtual_address: usize,
    should_allocate: bool,
) -> Result<usize> {
    if virtual_address > MAX_VA {
        bail!(error::Error::AddressOverflow);
    }

    let mut page_table_entry: usize;

    // Sv48 paging supports a 4 level page table (3, 2, 1, 0)
    for level in (1..4).rev() {
        unsafe {
            page_table_entry = (page_table as *const usize)
                .offset(extract_index_into_level(level, virtual_address) as isize)
                .addr(); // address of page table entry

            if (*(page_table_entry as *const usize) & VALID_BIT) == 0b1
            // if page table entry is valid
            {
                page_table =
                    page_table_entry_to_physical_address(*(page_table_entry as *const usize));
            } else {
                if should_allocate {
                    match state.allocate(PAGE_SIZE) {
                        Ok(v) => page_table = v,
                        Err(e) => return Err(e.into()),
                    }

                    write_volatile(
                        page_table_entry as *mut usize,
                        physical_address_to_page_table_entry(page_table) | VALID_BIT,
                    );
                } else {
                    bail!(error::Error::PageNotAllocated);
                }
            }
        }
    }
    unsafe {
        Ok((page_table as *const usize)
            .offset(extract_index_into_level(0, virtual_address) as isize)
            .addr())
    }
}

#[inline(always)]
pub fn kernel_stack_address(pid: usize) -> usize {
    TRAMPOLINE - 7 * (pid + 1) * PAGE_SIZE
}

pub fn map_kernel_stack(state: &GlobalState) {
    let mut physical_address: usize;

    for i in 0..MAXIMUM_PROCESS {
        match state.allocate(6 * PAGE_SIZE) {
            Ok(v) => physical_address = v,
            Err(_) => {
                panic!("ERROR - WHILE MAPPING KERNEL STACK - Page Fault: No Free Memory\n");
            }
        }
        unsafe {
            map(
                state,
                KERNEL_PAGE_TABLE,
                kernel_stack_address(i),
                physical_address,
                6 * PAGE_SIZE,
                READ_WRITE,
            )
            .unwrap()
        }
    }
}

pub static mut SUPERVISOR: bool = false;

/// Translates a virtual address to a physical address using the given page table.
pub fn translate_virtual_address(
    state: &GlobalState,
    page_table: usize,
    va: usize,
) -> Result<usize> {
    let aligned_virtual_address = (va / PAGE_SIZE) * PAGE_SIZE;
    let offset = va.saturating_sub(aligned_virtual_address);
    let page_table_entry_address = get_page_table_entry_address(state, page_table, va, false)?;
    let page_table_entry = unsafe { *(page_table_entry_address as *const usize) };

    if page_table_entry & VALID_BIT == 0 {
        bail!(error::Error::PageTableEntryInvalid);
    } else if page_table_entry & USER_MODE == 0 && !unsafe { SUPERVISOR } {
        bail!(error::Error::PageTableEntryNotAccessibleInUserMode);
    }

    Ok(page_table_entry_to_physical_address(page_table_entry) + offset)
}

pub fn allocate_heap(
    state: &GlobalState,
    increment: isize,
    trapframe: &TrapFrame,
) -> Result<usize> {
    if ((trapframe.brk.get() as isize + increment as isize) < 0)
        || (trapframe.brk.get() as i128 + increment as i128) >= isize::MAX as i128
    {
        bail!(error::Error::InvalidHeapSize)
    } else {
        let new_brk = trapframe.brk.get() as isize + increment;

        if new_brk >= trapframe.heap_end.get() as isize {
            let num_bytes = (new_brk - trapframe.heap_end.get() as isize) as usize;

            let num_pages = {
                if num_bytes == 0 {
                    1
                } else {
                    (num_bytes + PAGE_SIZE - 1) / PAGE_SIZE
                }
            };

            if (trapframe.heap_end.get() + num_pages * PAGE_SIZE) as i128
                >= (TRAMPOLINE - 12 * PAGE_SIZE) as i128
            {
                bail!(error::Error::InvalidHeapSize);
            }

            if (trapframe.heap_end.get() as i128 + (num_pages * PAGE_SIZE) as i128)
                >= isize::MAX as i128
            {
                bail!(error::Error::InvalidHeapSize);
            }

            let pa = state.allocate(num_pages * PAGE_SIZE)?;
            map(
                state,
                trapframe.page_table,
                trapframe.heap_end.get(),
                pa,
                num_pages * PAGE_SIZE,
                READ_WRITE | USER_MODE,
            )?;

            let old = trapframe.brk.get();

            trapframe
                .heap_end
                .set(trapframe.heap_end.get() + num_pages * PAGE_SIZE);

            let state = GlobalState::get();
            let process = state.get_current_process().unwrap();
            let mut current_process = process.lock();

            current_process.size = trapframe.heap_end.get();

            trapframe.brk.set(new_brk as usize);

            Ok(old)
        } else {
            let old = trapframe.brk.get();
            trapframe.brk.set(old + increment as usize);
            Ok(old)
        }
    }
}

#[inline(always)]
pub fn permissions_from_page_table_entry(page_table_entry: usize) -> usize {
    page_table_entry & 0b1111111111
}

pub fn copy(state: &GlobalState, old: usize, new: usize, size: usize) -> Result<()> {
    for page_va in (0..=size).step_by(PAGE_SIZE) {
        let pte_address = match get_page_table_entry_address(state, old, page_va, false) {
            Ok(v) => v,
            Err(e) if matches!(e.downcast_ref().unwrap(), Error::PageNotAllocated) => continue,
            Err(e) => return Err(e),
        };

        let pte = unsafe { *(pte_address as *const usize) };

        if pte & VALID_BIT == 0 {
            continue;
        }

        let pa = page_table_entry_to_physical_address(pte);
        let new_pa = state.allocate(PAGE_SIZE)?;

        unsafe {
            ptr::copy_nonoverlapping(pa as *const u8, new_pa as *mut u8, PAGE_SIZE);
        }

        map(
            state,
            new,
            page_va,
            new_pa,
            PAGE_SIZE,
            permissions_from_page_table_entry(pte),
        )?;
    }

    for page_va in (STACK_START..STACK_START + STACK_PAGES * PAGE_SIZE).step_by(PAGE_SIZE) {
        let pte_address = match get_page_table_entry_address(state, old, page_va, false) {
            Ok(v) => v,
            Err(_) => panic!(
                "ERROR WHILE COPYING PAGES IN FORK: STACK SHOULD BE ALLOCATED. THIS INDICATES A BUG"
            ),
        };

        let pte = unsafe { *(pte_address as *const usize) };

        if pte & VALID_BIT == 0 {
            continue;
        }

        let pa = page_table_entry_to_physical_address(pte);
        let new_pa = state.allocate(PAGE_SIZE)?;

        unsafe {
            ptr::copy_nonoverlapping(pa as *const u8, new_pa as *mut u8, PAGE_SIZE);
        }

        map(
            state,
            new,
            page_va,
            new_pa,
            PAGE_SIZE,
            permissions_from_page_table_entry(pte),
        )?;
    }

    Ok(())
}

/// Removes a mapping from the given page table, deallocating memory if necessary.
pub fn unmap_pages(
    state: &GlobalState,
    page_table: usize,
    va: usize,
    num_pages: usize,
    deallocate: bool,
) {
    for page_va in (va..va + num_pages * PAGE_SIZE).step_by(PAGE_SIZE) {
        if let Ok(pte_address) = get_page_table_entry_address(state, page_table, page_va, false) {
            let pte = unsafe { *(pte_address as *const usize) };

            if pte & VALID_BIT == 0 {
                continue;
            }

            if deallocate {
                state.deallocate(page_table_entry_to_physical_address(pte), PAGE_SIZE);
            }

            unsafe {
                *(pte_address as *mut usize) = 0;
            }
        }
    }
}

pub fn unmap_trampoline(state: &GlobalState, page_table: usize) -> Result<()> {
    let pte_address =
        get_page_table_entry_address(state, page_table, TRAMPOLINE, false)? as *mut usize;

    if unsafe { *pte_address } & VALID_BIT == 0 {
        panic!("NO TRAMPOLINE");
    }

    unsafe {
        *pte_address = 0;
    }

    Ok(())
}

/// Completely deallocates any pages of the page table and any pages pointed to by the page table.
/// The function first unmaps the pages, deallocating if necessary, and after all pages pointed to by
/// leaf page-table entries have been deallocated, the function deallocates the pages of the page table entries
/// themselves.
pub fn drop_pages(state: &GlobalState, page_table: usize, heap_end: usize) -> Result<()> {
    // unmap and deallocate PT_LOAD pages and heap pages
    unmap_pages(state, page_table, 0, heap_end / PAGE_SIZE, true);

    // unmap and deallocate user stack pages
    unmap_pages(state, page_table, STACK_START, STACK_PAGES, true);

    // unmap and deallocate trapframe
    unmap_pages(state, page_table, TRAPFRAME, 1, true);

    unmap_trampoline(state, page_table)?;

    free_page_table_recursive(state, page_table);

    Ok(())
}

/// Walks the page table recursively deallocating pages pointed to by each page table entry.
/// Analogous to xv6-riscv's `freewalk`.
pub fn free_page_table_recursive(state: &GlobalState, page_table: usize) {
    let page_table = page_table as *mut usize;

    for i in 0..512 {
        let page_table_entry = unsafe { *page_table.offset(i) };

        if page_table_entry & VALID_BIT != 0 {
            if page_table_entry & (READ_ONLY | WRITE_ONLY | EXECUTE_ONLY) == 0 {
                free_page_table_recursive(
                    state,
                    page_table_entry_to_physical_address(page_table_entry),
                );
            } else {
                panic!("LEAF PAGE TABLE");
            }
        }

        unsafe { *page_table.offset(i) = 0 };
    }

    state.deallocate(page_table.addr(), PAGE_SIZE);
}
