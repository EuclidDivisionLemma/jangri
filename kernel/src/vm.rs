use crate::{
    allocator::allocate,
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_PAGE_TABLE, KERNEL_START, MAX_VA, MAXIMUM_PROCESS, PAGE_SIZE,
        PLIC, PLIC_SIZE, RAM_STOP, READ_EXECUTE, READ_WRITE, TRAMPOLINE, TRAMPOLINE_CODE_ADDRESS,
        UART0, USER_MODE, VALID_BIT, VIRTIO_MMIO_DISK, VIRTIO_MMIO_DISK_SIZE,
    },
    error::{self, Result},
};
use core::{arch::asm, f64::math::floor, ptr::read_volatile};
use core::{f64::math::ceil, ptr::write_volatile};

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
    ceil(size as f64 / PAGE_SIZE as f64) as usize * PAGE_SIZE
}

pub fn initialise_kernel_page_table() -> Result<()> {
    unsafe {
        KERNEL_PAGE_TABLE = allocate(1)?;

        map_kernel_stack();

        // map UART registers
        map(KERNEL_PAGE_TABLE, UART0, UART0, PAGE_SIZE, READ_WRITE)?;

        // map VIRIO MMIO Disk Registers
        map(
            KERNEL_PAGE_TABLE,
            VIRTIO_MMIO_DISK,
            VIRTIO_MMIO_DISK,
            VIRTIO_MMIO_DISK_SIZE,
            READ_WRITE,
        )?;

        // map PLIC Registers
        map(KERNEL_PAGE_TABLE, PLIC, PLIC, PLIC_SIZE, READ_WRITE)?;

        // map kernel code
        map(
            KERNEL_PAGE_TABLE,
            KERNEL_START,
            KERNEL_START,
            END_OF_KERNEL_TEXT - KERNEL_START,
            READ_EXECUTE,
        )?;

        // map kernel data and RAM
        map(
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
    page_table: usize,
    virtual_address: usize,
    physical_address: usize,
    size: usize,
    permissions: usize,
) -> Result<()> {
    map_pages(
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
    page_table: usize,
    virtual_address: usize,
    physical_address: usize,
    size: usize,
    permissions: usize,
) -> Result<()> {
    map_pages(
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
    page_table: usize,
    mut virtual_address: usize,
    mut physical_address: usize,
    size: usize,
    permissions: usize,
    trampoline: bool,
) -> Result<()> {
    let mut page_table_entry_address: usize;

    if virtual_address % PAGE_SIZE != 0 {
        return Err(error::Error::VirtualAddressMisaligned);
    }

    if size % PAGE_SIZE != 0 {
        return Err(error::Error::SizeMisaligned);
    }

    if size == 0 {
        return Err(error::Error::ZeroSize);
    }

    let number_of_pages = size / PAGE_SIZE;

    for _ in 0..number_of_pages {
        match get_page_table_entry_address(page_table, virtual_address, true) {
            Ok(v) => page_table_entry_address = v,
            Err(e) => return Err(e),
        }

        unsafe {
            if read_volatile(page_table_entry_address as *const usize) & VALID_BIT == 0b1 {
                return Err(error::Error::ValidPageRemap);
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
    mut page_table: usize,
    virtual_address: usize,
    should_allocate: bool,
) -> Result<usize> {
    if virtual_address > MAX_VA {
        return Err(error::Error::AddressOverflow);
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
                    match allocate(1) {
                        Ok(v) => page_table = v,
                        Err(e) => return Err(e),
                    }

                    write_volatile(
                        page_table_entry as *mut usize,
                        physical_address_to_page_table_entry(page_table) | VALID_BIT,
                    );
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

pub fn map_kernel_stack() {
    let mut physical_address: usize;

    for i in 0..MAXIMUM_PROCESS {
        match allocate(1) {
            Ok(v) => physical_address = v,
            Err(_) => {
                panic!("ERROR - WHILE MAPPING KERNEL STACK - Page Fault: No Free Memory\n");
            }
        }
        unsafe {
            map(
                KERNEL_PAGE_TABLE,
                TRAMPOLINE - (i + 1) * 2 * PAGE_SIZE,
                physical_address,
                PAGE_SIZE,
                READ_WRITE,
            )
            .unwrap()
        }
    }
}

/// Translates a virtual address to a physical address using the given page table.
pub fn translate_virtual_address(page_table: usize, va: usize) -> Result<usize> {
    let aligned_virtual_address = (va / PAGE_SIZE) * PAGE_SIZE;
    let offset = va.saturating_sub(aligned_virtual_address);
    let page_table_entry_address = get_page_table_entry_address(page_table, va, false)?;
    let page_table_entry = unsafe { *(page_table_entry_address as *const usize) };

    if page_table_entry & VALID_BIT == 0 {
        return Err(error::Error::PageTableEntryInvalid);
    } else if page_table_entry & USER_MODE == 0 {
        return Err(error::Error::PageTableEntryNotAccessibleInUserMode);
    }

    Ok(page_table_entry_to_physical_address(page_table_entry) + offset)
}

pub fn copy_from_va(dest: *mut u8, src: *mut u8, page_table: usize) {}
