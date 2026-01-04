use crate::{
    DEVICE,
    allocator::allocate,
    constants::{
        EXECUTE_ONLY, KERNEL_PAGE_TABLE, MAXIMUM_PROCESS, PAGE_SIZE, READ_EXECUTE, READ_ONLY,
        READ_WRITE, ROOT_INODE, STACK_PAGES, STACK_START, Sv48, TRAMPOLINE,
        TRAMPOLINE_CODE_ADDRESS, TRAMPOLINE_OFFSET, TRAPFRAME, USER_MODE, WRITE_ONLY,
    },
    error::{Error, Result},
    fs::sfs::{MemoryINode, read_inode},
    scheduler::{Context, switch_to_scheduler_context},
    traps::{TrapFrame, set_up_supervisor_to_user_mode_transition, user_trap},
    vm::{self, kernel_stack_address, map, map_trampoline},
};
use alloc::{
    boxed::Box,
    rc::Rc,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{arch::global_asm, cell::LazyCell, f64::math::ceil, mem::transmute, ptr};
use elf::{ElfBytes, endian::NativeEndian};

global_asm!(
    r#"
    .section .rodata
    .global process_1_start
    .global process_1_end

    process_1_start:
        .incbin "../userspace/sh.elf"
    process_1_end:
    "#
);

global_asm!(
    r#"
    .section .rodata
    .global process_2_start
    .global process_2_end

    process_2_start:
        .incbin "../build/process2"
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

#[derive(Clone)]
pub enum ProcessState {
    Ready { cwd: Rc<MemoryINode> },
    Running { cwd: Rc<MemoryINode> },
    Waiting,
    Terminated { return_value: isize },
    NotUsed,
    Sleeping { cwd: Rc<MemoryINode> },
}

pub struct Process<'a> {
    pub id: usize,
    name: String,

    /// CAUTION: Holds the bottom of the stack
    kernel_stack: usize,
    pub state: ProcessState,
    pub context: Context,
    parent: Option<&'a Process<'a>>,
    children: Option<Vec<Arc<Process<'a>>>>,
    pub page_table: usize,
    pub code: usize,
    pub trapframe: Option<Box<TrapFrame>>,
    pub sleep_on: Option<usize>,
    pub fds: Vec<usize>,
    pub size: usize,
}

impl<'a> Process<'a> {
    fn default(id: usize) -> Self {
        Process {
            id,
            name: "".into(),
            kernel_stack: 0,
            state: ProcessState::NotUsed,
            context: Context::default(),
            parent: None,
            children: None,
            page_table: 0,
            code: 0,
            trapframe: None,
            sleep_on: None,
            fds: Vec::new(),
            size: 0,
        }
    }

    pub fn clone(&'static mut self) -> Result<&'a mut Self> {
        let process = assign_process()?;
        process.context = self.context.clone();

        process.fds = self.fds.clone();
        process.kernel_stack = kernel_stack_address(process.id);
        process.context.sp = process.kernel_stack + PAGE_SIZE;
        process.context.ra = prepare_first_time_execution as usize;

        if let Some(child_trapframe) = process.trapframe.as_mut()
            && let Some(parent_trapframe) = self.trapframe.as_mut()
        {
            *child_trapframe = (*parent_trapframe).clone();
            child_trapframe.a0 = 0;
            parent_trapframe.a0 = process.id;

            child_trapframe.page_table = process.page_table;
            child_trapframe.kernel_stack = process.kernel_stack;
            child_trapframe.satp = Sv48 | (child_trapframe.page_table >> 12);

            map(
                process.page_table,
                TRAPFRAME,
                (&raw const **child_trapframe).addr(),
                PAGE_SIZE,
                READ_WRITE,
            )?;
        }

        process.parent = Some(&*self);

        process.name = self.name.clone();

        if let ProcessState::Running { cwd } = &self.state {
            process.state = ProcessState::Ready { cwd: cwd.clone() };
        }

        vm::copy(self.page_table, process.page_table, self.size)?;

        map_trampoline(
            process.page_table,
            TRAMPOLINE,
            unsafe { TRAMPOLINE_CODE_ADDRESS },
            PAGE_SIZE,
            READ_EXECUTE,
        )?;

        Ok(process)
    }
}

pub fn yield_cpu() {
    let process = unsafe {
        &mut **CURRENT_PROCESS
            .as_mut()
            .expect("YIELD FAILED - NO CURRENT PROCESS")
    };

    if let ProcessState::Running { cwd } = &process.state {
        process.state = ProcessState::Ready { cwd: cwd.clone() }
    }
    switch_to_scheduler_context();
}

pub fn assign_process() -> Result<&'static mut Process<'static>> {
    let page_table = allocate(1)?;
    let trapframe = allocate(1)?;

    for process in unsafe { &mut *PROCESSES } {
        if let ProcessState::NotUsed = process.state {
            process.state = ProcessState::Ready {
                cwd: read_inode(ROOT_INODE, &DEVICE),
            };
            process.page_table = page_table;

            process.trapframe = Some(unsafe { Box::from_raw(trapframe as *mut TrapFrame) });
            **process.trapframe.as_mut().unwrap() = TrapFrame::default();
            process.trapframe.as_mut().unwrap().page_table = page_table;
            process.trapframe.as_mut().unwrap().satp = Sv48 | page_table >> 12;
            process.trapframe.as_mut().unwrap().kernel_page_table =
                Sv48 | (unsafe { KERNEL_PAGE_TABLE } >> 12);
            process.trapframe.as_mut().unwrap().user_trap_address = user_trap as usize;

            return Ok(process);
        }
    }

    Err(Error::NoUnusedProcess)
}

pub fn map_code_pages(
    page_table: usize,
    code_pa: usize,
    code_va: usize,
    num_code_pages: usize,
    permissions: usize,
) {
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
        permissions | USER_MODE,
    )
    .expect("PROCESS CREATION FAILED - ERROR WHILE MAPPING CODE PAGES");
}

pub fn map_other_pages(page_table: usize, final_code: usize, process: &mut Process) -> Result<()> {
    if (TRAPFRAME - final_code) < (14 * PAGE_SIZE) {
        panic!("PROCESS CREATION FAILED - NOT ENOUGH SPACE FOR STACK AND HEAP");
    }

    let stack = allocate(STACK_PAGES)?;

    map(
        page_table,
        STACK_START,
        stack,
        3 * PAGE_SIZE,
        READ_WRITE | USER_MODE,
    )?;

    process.kernel_stack = kernel_stack_address(process.id);

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
    trapframe.sp = TRAMPOLINE - 9 * PAGE_SIZE;

    Ok(())
}

pub fn start_init_1() {
    let start = unsafe { &process_1_start as *const usize as usize };
    let end = unsafe { &process_1_end as *const usize as usize };
    let size = end - start;
    let mut page = 0;
    let mut max_code_page_end_va = 0;

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

            let offset = header.p_offset as usize;

            let va = header.p_vaddr as usize;
            let flags = header.p_flags;

            if va + num_pages * PAGE_SIZE > max_code_page_end_va {
                max_code_page_end_va = page + num_pages * PAGE_SIZE;
            }

            let mut permissions = 0;

            if flags & elf::abi::PF_R != 0 {
                permissions |= READ_ONLY;
            }

            if flags & elf::abi::PF_W != 0 {
                permissions |= WRITE_ONLY;
            }

            if flags & elf::abi::PF_X != 0 {
                permissions |= EXECUTE_ONLY;
            }

            let loadable = &elf_bytes[offset..offset + file_size];

            map_code_pages(process.page_table, page, va, num_pages, permissions);
            process.size += num_pages * PAGE_SIZE;

            unsafe {
                ptr::copy_nonoverlapping(loadable.as_ptr(), page as *mut u8, file_size);
            }
        }
    }

    if page == 0 {
        panic!("PANIC: INIT FAILED - ELF CONTAINS NO LOADABLE SEGMENT");
    }

    map_other_pages(process.page_table, max_code_page_end_va, process)
        .expect("INIT FAILED - ERROR WHILE MAPPING PAGES");

    let trapframe = process
        .trapframe
        .as_mut()
        .expect("TRAPFRAME NONE IN FIRST TIME EXECUTION");

    trapframe.sepc = elf_data.ehdr.e_entry as usize;
    process.context.ra = prepare_first_time_execution as usize;

    process.context.sp = trapframe.kernel_stack + PAGE_SIZE;
    trapframe.brk.set(max_code_page_end_va);
    trapframe.heap_end.set(max_code_page_end_va);
}

/// This function is called when a process has to be executed for the first time.
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

impl<'a> Process<'a> {
    pub fn sleep(&mut self, sleep_on: usize) {
        self.sleep_on = Some(sleep_on);

        if let ProcessState::Ready { cwd } = &self.state {
            self.state = ProcessState::Sleeping { cwd: cwd.clone() }
        }
    }
}

pub fn wake_up(sleep_on: usize) {
    for process in unsafe { &mut *PROCESSES } {
        if let Some(v) = process.sleep_on {
            if v == sleep_on {
                process.sleep_on = None;

                if let ProcessState::Sleeping { cwd } = &process.state {
                    process.state = ProcessState::Ready { cwd: cwd.clone() };
                }
            }
        }
    }
}
