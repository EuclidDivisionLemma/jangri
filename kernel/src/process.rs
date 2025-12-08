use crate::{
    allocator::allocate,
    constants::{
        HEAP_PAGES, KERNEL_PAGE_TABLE, MAXIMUM_PROCESS, PAGE_SIZE, READ_EXECUTE, READ_WRITE,
        READ_WRITE_EXECUTE, STACK_PAGES, Sv48, TRAMPOLINE, TRAMPOLINE_CODE_ADDRESS,
        TRAMPOLINE_OFFSET, TRAPFRAME, USER_MODE,
    },
    error::{Error, Result},
    scheduler::{Context, switch_to_scheduler_context},
    syscall::stdout,
    traps::{TrapFrame, set_up_supervisor_to_user_mode_transition, user_trap},
    vm::{map, map_trampoline},
};
use alloc::{
    boxed::Box,
    format,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{
    arch::{asm, global_asm},
    cell::LazyCell,
    f64::math::ceil,
    mem::transmute,
    ptr,
};
use elf::{ElfBytes, endian::NativeEndian};
use riscv::register::satp::{Mode, Satp};

global_asm!(
    r#"
    .section .rodata
    .global process_1_start
    .global process_1_end

    process_1_start:
        .incbin "target/process1.bin"
    process_1_end:
    "#
);

global_asm!(
    r#"
    .section .rodata
    .global process_2_start
    .global process_2_end

    process_2_start:
        .incbin "target/process2.bin"
    process_2_end:
    "#
);

unsafe extern "C" {
    static process_1_start: usize;
    static process_1_end: usize;
    static process_2_start: usize;
    static process_2_end: usize;
}

pub static mut PROCESSES: LazyCell<[Process; MAXIMUM_PROCESS]> =
    LazyCell::new(|| core::array::from_fn(|i| Process::default(i)));

pub static mut CURRENT_PROCESS: Option<&'static mut Process> = None;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
    NotUsed,
}

pub struct Process<'a> {
    pub id: usize,
    name: &'a str,
    kernel_stack: usize,
    pub state: ProcessState,
    pub context: Context,
    parent: Option<Weak<Process<'a>>>,
    children: Option<Vec<Arc<Process<'a>>>>,
    pub page_table: usize,
    pub code: usize,
    pub trapframe: Option<Box<TrapFrame>>,
}

impl<'a> Process<'a> {
    fn default(id: usize) -> Self {
        Process {
            id,
            name: "",
            kernel_stack: 0,
            state: ProcessState::NotUsed,
            context: Context::default(),
            parent: None,
            children: None,
            page_table: 0,
            code: 0,
            trapframe: None,
        }
    }
}

pub fn yield_cpu() {
    let process = unsafe {
        &mut **CURRENT_PROCESS
            .as_mut()
            .expect("YIELD FAILED - NO CURRENT PROCESS")
    };

    process.state = ProcessState::Ready;
    switch_to_scheduler_context();
}

#[unsafe(no_mangle)]
pub fn assign_process() -> Result<&'static mut Process<'static>> {
    let page_table = allocate(1)?;
    let trapframe = allocate(1)?;

    for process in unsafe { &mut *PROCESSES } {
        if process.state == ProcessState::NotUsed {
            process.state = ProcessState::Ready;
            process.page_table = page_table;

            process.trapframe = Some(unsafe { Box::from_raw(trapframe as *mut TrapFrame) });
            **process.trapframe.as_mut().unwrap() = TrapFrame::default();
            process.trapframe.as_mut().unwrap().page_table = Sv48 | (page_table >> 12);
            process.trapframe.as_mut().unwrap().kernel_page_table =
                Sv48 | (unsafe { KERNEL_PAGE_TABLE } >> 12);
            process.trapframe.as_mut().unwrap().user_trap_address = user_trap as usize;

            return Ok(process);
        }
    }

    Err(Error::NoUnusedProcess)
}

#[unsafe(no_mangle)]
pub fn map_code_pages(page_table: usize, code_pa: usize, code_va: usize, num_code_pages: usize) {
    if code_va == TRAMPOLINE {
        panic!("PROCESS CREATION FAILED - CODE SEGMENT CANNOT BE MAPPED TO TRAMPOLINE ADDRESS");
    } else if code_va == TRAPFRAME {
        panic!("PROCESS CREATION FAILED - CODE SEGMENT CANNOT BE MAPPED TO TRAPFRAME ADDRESS");
    }
    map(
        page_table,
        code_va,
        code_pa,
        num_code_pages * PAGE_SIZE,
        READ_EXECUTE | USER_MODE,
    )
    .expect("PROCESS CREATION FAILED - ERROR WHILE MAPPING CODE PAGES");
}

pub fn map_other_pages(page_table: usize, final_code: usize, process: &mut Process) -> Result<()> {
    if (TRAPFRAME - final_code) < (14 * PAGE_SIZE) {
        panic!("PROCESS CREATION FAILED - NOT ENOUGH SPACE FOR STACK AND HEAP");
    }

    let stack = allocate(STACK_PAGES)?;
    let heap = allocate(HEAP_PAGES)?;
    let kernel_stack = allocate(1)?;

    map(
        page_table,
        TRAMPOLINE - 7 * PAGE_SIZE,
        stack,
        3 * PAGE_SIZE,
        READ_WRITE | USER_MODE,
    )?;

    map(
        page_table,
        TRAMPOLINE - 18 * PAGE_SIZE,
        heap,
        10 * PAGE_SIZE,
        READ_WRITE | USER_MODE,
    )?;

    process.kernel_stack = kernel_stack;

    map(
        page_table,
        TRAMPOLINE - 3 * PAGE_SIZE,
        process.kernel_stack,
        PAGE_SIZE,
        READ_WRITE,
    )?;

    map_trampoline(
        page_table,
        TRAMPOLINE,
        unsafe { TRAMPOLINE_CODE_ADDRESS },
        PAGE_SIZE,
        READ_EXECUTE,
    )?;

    let trapframe = process.trapframe.as_mut().ok_or(Error::TrapFrameNone)?;

    map(
        page_table,
        TRAPFRAME,
        (&raw const **trapframe).addr(),
        PAGE_SIZE,
        READ_WRITE,
    )?;

    trapframe.kernel_page_table = Sv48 | (unsafe { KERNEL_PAGE_TABLE } >> 12);
    trapframe.kernel_stack = process.kernel_stack;
    trapframe.sp = TRAMPOLINE - 4 * PAGE_SIZE;

    Ok(())
}

#[unsafe(no_mangle)]
pub fn start_init_1() {
    let start = unsafe { &process_1_start as *const usize as usize };
    let end = unsafe { &process_1_end as *const usize as usize };
    let size = end - start;
    let mut page = 0;
    let mut max_code_page_end = 0;

    let process = assign_process().expect("INIT FAILED - FAILED TO ASSIGN PROCESS");

    let elf_bytes = unsafe { core::slice::from_raw_parts(start as *const u8, size) };
    let elf_data: ElfBytes<NativeEndian> =
        elf::ElfBytes::minimal_parse(elf_bytes).expect("INIT FAILED - ELF ERROR");
    let program_headers = elf_data
        .segments()
        .expect("INIT FAILED - NO SEGMENTS")
        .iter();

    for header in program_headers {
        if header.p_type == elf::abi::PT_LOAD {
            let file_size = header.p_filesz as usize;
            let mem_size = header.p_memsz as usize;

            let num_pages = ceil(mem_size as f64 / PAGE_SIZE as f64) as usize;

            page = allocate(num_pages).expect("INIT FAILED - FAILED TO ALLOCATE PAGE FOR CODE");

            if page > max_code_page_end {
                max_code_page_end = page + num_pages * PAGE_SIZE;
            }

            let offset = header.p_offset as usize;

            let va = header.p_vaddr as usize;
            let flags = header.p_flags;

            let loadable = &elf_bytes[offset..offset + file_size];

            map_code_pages(process.page_table, page, va, num_pages);

            unsafe {
                ptr::copy_nonoverlapping(loadable.as_ptr(), page as *mut u8, file_size);
            }
        }
    }

    if page == 0 {
        panic!("PANIC: INIT FAILED - ELF CONTAINS NO LOADABLE SEGMENT");
    }

    map_other_pages(process.page_table, max_code_page_end, process)
        .expect("INIT FAILED - ERROR WHILE MAPPING PAGES");

    let trapframe = process
        .trapframe
        .as_mut()
        .expect("TRAPFRAME NONE IN FIRST TIME EXECUTION");

    trapframe.sepc = elf_data.ehdr.e_entry as usize;
    process.context.ra = prepare_first_time_execution as usize;

    process.context.sp = trapframe.kernel_stack + PAGE_SIZE;
}

pub fn start_init_2() {
    let start = unsafe { &process_2_start as *const usize as usize };
    let end = unsafe { &process_2_end as *const usize as usize };
    let size = end - start;
    let mut page = 0;
    let mut max_code_page_end = 0;

    let process = assign_process().expect("INIT FAILED - FAILED TO ASSIGN PROCESS");

    let elf_bytes = unsafe { core::slice::from_raw_parts(start as *const u8, size) };
    let elf_data: ElfBytes<NativeEndian> =
        elf::ElfBytes::minimal_parse(elf_bytes).expect("INIT FAILED - ELF ERROR");
    let program_headers = elf_data
        .segments()
        .expect("INIT FAILED - NO SEGMENTS")
        .iter();

    for header in program_headers {
        if header.p_type == elf::abi::PT_LOAD {
            let file_size = header.p_filesz as usize;
            let mem_size = header.p_memsz as usize;

            let num_pages = ceil(mem_size as f64 / PAGE_SIZE as f64) as usize;

            page = allocate(num_pages).expect("INIT FAILED - FAILED TO ALLOCATE PAGE FOR CODE");

            if page > max_code_page_end {
                max_code_page_end = page + num_pages * PAGE_SIZE;
            }

            let offset = header.p_offset as usize;

            let va = header.p_vaddr as usize;
            let flags = header.p_flags;

            let loadable = &elf_bytes[offset..offset + file_size];

            map_code_pages(process.page_table, page, va, num_pages);

            unsafe {
                ptr::copy_nonoverlapping(loadable.as_ptr(), page as *mut u8, file_size);
            }
        }
    }

    if page == 0 {
        panic!("PANIC: INIT FAILED - ELF CONTAINS NO LOADABLE SEGMENT");
    }

    map_other_pages(process.page_table, max_code_page_end, process)
        .expect("INIT FAILED - ERROR WHILE MAPPING PAGES");

    let trapframe = process
        .trapframe
        .as_mut()
        .expect("TRAPFRAME NONE IN FIRST TIME EXECUTION");

    trapframe.sepc = elf_data.ehdr.e_entry as usize;
    process.context.ra = prepare_first_time_execution as usize;

    process.context.sp = trapframe.kernel_stack + PAGE_SIZE;
}

/// This function is called when a process has to be executed for the first time.
#[unsafe(no_mangle)]
pub fn prepare_first_time_execution() {
    let process = unsafe {
        &mut **CURRENT_PROCESS
            .as_mut()
            .expect("INIT FAILED - NO CURRENT PROCESS IN FIRST TIME EXECUTION")
    };

    let trapframe = process
        .trapframe
        .as_mut()
        .expect("INIT FAILED - TRAPFRAME NONE IN FIRST TIME EXECUTION");

    set_up_supervisor_to_user_mode_transition()
        .expect("INIT FAILED - CONTEXT NONE WHILE RETURNING TO USER MODE");

    unsafe {
        let return_to_user_mode_ptr: fn(usize) -> ! = transmute(TRAMPOLINE + TRAMPOLINE_OFFSET);
        return_to_user_mode_ptr((&raw const **trapframe).addr());
    }
}
