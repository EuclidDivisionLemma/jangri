use crate::{
    ARCH, PAGE_TABLE_ENTRY,
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_PAGE_TABLE, KERNEL_START, MAXIMUM_PROCESS, PLIC, PLIC_SIZE,
        RAM_STOP, STACK_PAGES, STACK_START, TRAMPOLINE, TRAMPOLINE_CODE_ADDRESS, TRAPFRAME, UART0,
        VIRTIO_MMIO_DISK, VIRTIO_MMIO_DISK_SIZE,
    },
    global_state::GlobalState,
};
use anyhow::Result;
use hal::constants::PAGE_SIZE;

pub fn enable_paging() {
    <ARCH as hal::vm::VirtualMemory<PAGE_TABLE_ENTRY>>::enable_paging(unsafe { KERNEL_PAGE_TABLE });
}

pub fn align_to_page_size(size: usize) -> usize {
    (size + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE
}

pub fn initialise_kernel_page_table(state: &GlobalState) -> Result<()> {
    unsafe {
        KERNEL_PAGE_TABLE = state.allocate(PAGE_SIZE).unwrap();

        map_kernel_stack(state);

        state.map(
            KERNEL_PAGE_TABLE,
            UART0,
            UART0,
            PAGE_SIZE,
            true,
            true,
            false,
            false,
        )?;

        state.map(
            KERNEL_PAGE_TABLE,
            VIRTIO_MMIO_DISK,
            VIRTIO_MMIO_DISK,
            VIRTIO_MMIO_DISK_SIZE,
            true,
            true,
            false,
            false,
        )?;

        state.map(
            KERNEL_PAGE_TABLE,
            PLIC,
            PLIC,
            PLIC_SIZE,
            true,
            true,
            false,
            false,
        )?;

        // map kernel code
        state.map(
            KERNEL_PAGE_TABLE,
            KERNEL_START,
            KERNEL_START,
            END_OF_KERNEL_TEXT - KERNEL_START,
            true,
            false,
            true,
            false,
        )?;

        // map kernel data and RAM
        state.map(
            KERNEL_PAGE_TABLE,
            END_OF_KERNEL_TEXT,
            END_OF_KERNEL_TEXT,
            RAM_STOP - END_OF_KERNEL_TEXT,
            true,
            true,
            false,
            false,
        )?;

        state.map(
            KERNEL_PAGE_TABLE,
            TRAMPOLINE,
            TRAMPOLINE_CODE_ADDRESS,
            PAGE_SIZE,
            true,
            false,
            true,
            false,
        )?;
    }

    Ok(())
}

#[inline(always)]
pub fn kernel_stack_address(pid: usize) -> usize {
    TRAMPOLINE - 5 * (pid + 1) * PAGE_SIZE
}

pub fn map_kernel_stack(state: &GlobalState) {
    let mut physical_address: usize;

    for i in 0..MAXIMUM_PROCESS {
        match state.allocate(4 * PAGE_SIZE) {
            Ok(v) => physical_address = v,
            Err(_) => {
                panic!("ERROR - WHILE MAPPING KERNEL STACK - Page Fault: No Free Memory\n");
            }
        }
        unsafe {
            state
                .map(
                    KERNEL_PAGE_TABLE,
                    kernel_stack_address(i),
                    physical_address,
                    4 * PAGE_SIZE,
                    true,
                    true,
                    false,
                    false,
                )
                .unwrap()
        }
    }
}

/// Completely deallocates any pages of the page table and any pages pointed to by the page table.
/// The function first unmaps the pages, deallocating if necessary, and after all pages pointed to by
/// leaf page-table entries have been deallocated, the function deallocates the pages of the page table entries
/// themselves.
pub fn drop_pages(state: &GlobalState, page_table: usize, heap_end: usize) -> Result<()> {
    // unmap and deallocate PT_LOAD pages and heap pages
    state.unmap(page_table, 0, heap_end / PAGE_SIZE, true)?;

    state.unmap(page_table, STACK_START, STACK_PAGES, true)?;

    state.unmap(page_table, TRAPFRAME, 1, true)?;

    state.unmap(page_table, TRAMPOLINE, 1, false)?;

    state.cleanup_page_table(page_table)?;

    Ok(())
}
