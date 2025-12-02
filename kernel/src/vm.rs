use crate::{
    allocator::allocate,
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_PAGE_TABLE, KERNEL_START, MAX_VA, MAXIMUM_PROCESS, PAGE_SIZE,
        PLIC, PLIC_SIZE, RAM_STOP, READ_EXECUTE, READ_WRITE, TRAMPOLINE, UART0, VALID_BIT,
        VIRTIO_MMIO_DISK, VIRTIO_MMIO_DISK_SIZE,
    },
    error::{self, Result},
    syscall::stdout,
};
use core::ptr::read_volatile;
use core::ptr::write_volatile;

macro_rules! extract_index_into_level {
    ($level: expr, $virtual_address: expr) => {
        $virtual_address >> (12 + ($level * 9)) & 0b111111111
    };
}

macro_rules! page_table_entry_to_physical_address {
    ($page_table_entry: expr) => {
        ($page_table_entry >> 10) << 12
    };
}

macro_rules! physical_address_to_page_table_entry {
    ($physical_address: expr) => {
        ($physical_address >> 12) << 10
    };
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

        // map trampoline (todo)
        // map(
        //     KERNEL_PAGE_TABLE,
        //     TRAMPOLINE,
        //     TRAMPOLINE_CODE_ADDRESS,
        //     PAGE_SIZE,
        //     READ_EXECUTE,
        // )?;
    }

    Ok(())
}

pub unsafe fn map(
    page_table: usize,
    mut virtual_address: usize,
    mut physical_address: usize,
    size: usize,
    permissions: usize,
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
        match get_page_table_entry_address(page_table, virtual_address) {
            Ok(v) => page_table_entry_address = v,
            Err(e) => return Err(e),
        }

        unsafe {
            if read_volatile(page_table_entry_address as *const usize) & VALID_BIT == 0b1 {
                return Err(error::Error::ValidPageRemap);
            }

            write_volatile(
                page_table_entry_address as *mut usize,
                physical_address_to_page_table_entry!(physical_address) | permissions | VALID_BIT,
            );
        }

        virtual_address += PAGE_SIZE;
        physical_address += PAGE_SIZE;
    }

    Ok(())
}

pub fn get_page_table_entry_address(
    mut page_table: usize,
    virtual_address: usize,
) -> Result<usize> {
    if virtual_address > MAX_VA {
        return Err(error::Error::AddressOverflow);
    }

    let mut page_table_entry: usize;

    // Sv48 paging supports a 4 level page table (3, 2, 1, 0)
    for level in (1..4).rev() {
        unsafe {
            page_table_entry = (page_table as *const usize)
                .offset(extract_index_into_level!(level, virtual_address) as isize)
                .addr(); // address of page table entry

            if (*(page_table_entry as *const usize) & VALID_BIT) == 0b1
            // if page table entry is valid
            {
                page_table =
                    page_table_entry_to_physical_address!(*(page_table_entry as *const usize));
            } else {
                match allocate(1) {
                    Ok(v) => page_table = v,
                    Err(e) => return Err(e),
                }

                write_volatile(
                    page_table_entry as *mut usize,
                    physical_address_to_page_table_entry!(page_table) | VALID_BIT,
                );
            }
        }
    }
    unsafe {
        Ok((page_table as *const usize)
            .offset(extract_index_into_level!(0, virtual_address) as isize)
            .addr())
    }
}

pub fn map_kernel_stack() {
    let mut physical_address: usize;

    for i in 0..MAXIMUM_PROCESS {
        match allocate(1) {
            Ok(v) => physical_address = v,
            Err(_) => {
                return stdout("Page Fault: No Free Memory\n");
            }
        }
        unsafe {
            match map(
                KERNEL_PAGE_TABLE,
                TRAMPOLINE - (i + 1) * 2 * PAGE_SIZE,
                physical_address,
                PAGE_SIZE,
                READ_WRITE,
            ) {
                Ok(_) => (),
                Err(e) => e.log(true),
            }
        }
    }
}
