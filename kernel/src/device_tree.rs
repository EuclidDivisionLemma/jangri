use core::arch::asm;

use crate::ARCH;
use crate::{print, println};
use alloc::{format, vec::Vec};
use fdt::{Fdt, node::NodeProperty, standard_nodes::MemoryRegion};
use hal::Hal;
use riscv_arch::uart;
use spin::once::Once;

static FDT: Once<Fdt> = Once::new();
static CPUS: Once<[usize; 4]> = Once::new();
pub static NUM_CPUS: Once<usize> = Once::new();

pub fn initialise() {
    let addr: usize;
    unsafe { asm!("mv {}, a1", out(reg) addr) };
    FDT.call_once(|| unsafe { Fdt::from_ptr(addr as *const u8).unwrap() });
}

pub fn find_cpus() {
    let mut cpus = [0; 4];
    let fdt = FDT.get().unwrap();
    let node = fdt.find_node("/cpus").unwrap();
    let mut i = 0;
    let address_cells = node.cell_sizes().address_cells;
    for child in node.children() {
        if let Some(prop) = child.property("device_type")
            && prop.value == b"cpu\0"
        {
            if let Some(prop) = child.property("status")
                && (prop.value == b"disabled\0" || prop.name == "fail\0")
            {
                continue;
            }
            let regs = child.reg().unwrap().collect::<Vec<MemoryRegion>>();
            for reg in regs {
                if i == 4 {
                    println!("PANIC: Maximum supported CPU count is 4, found more; Halting");
                    loop {}
                }
                cpus[i] = reg.starting_address as usize;
                i += 1;
            }
        }
    }

    CPUS.call_once(|| cpus);
    NUM_CPUS.call_once(|| i);
}

pub fn cpu_mask() -> u8 {
    let mut mask = 0;
    for cpu_id in CPUS
        .get()
        .unwrap()
        .iter()
        .filter_map(|e| if *e != usize::MAX { Some(e) } else { None })
    {
        mask |= 1 << cpu_id;
    }
    mask
}
