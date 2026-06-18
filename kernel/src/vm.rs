use crate::{
    ARCH, PAGE_TABLE_ENTRY,
    constants::{
        END_OF_KERNEL_TEXT, KERNEL_PAGE_TABLE, KERNEL_START, MAXIMUM_PROCESS, PLIC, PLIC_SIZE,
        RAM_STOP, TRAMPOLINE_CODE_ADDRESS, UART0,
    },
    global_state::GlobalState,
};
use hal::constants::{PAGE_SIZE, TRAMPOLINE};
use hal::error::Result;

pub fn enable_paging() {
    <ARCH as hal::vm::VirtualMemory<PAGE_TABLE_ENTRY>>::enable_paging(unsafe { KERNEL_PAGE_TABLE });
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
