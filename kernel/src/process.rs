use crate::{
    ARCH, DEVICE, INIT, Mutex, PAGE_TABLE_ENTRY, TrapFrame,
    constants::{
        EXECUTE_ONLY, KERNEL_PAGE_TABLE, MAXIMUM_PROCESS, PAGE_SIZE, READ_EXECUTE, READ_ONLY,
        READ_WRITE, ROOT_INODE, STACK_PAGES, STACK_START, Sv48, TRAMPOLINE,
        TRAMPOLINE_CODE_ADDRESS, TRAMPOLINE_OFFSET, TRAPFRAME, USER_MODE, WRITE_ONLY,
    },
    error::Error,
    fs::sfs::{MemoryINode, read_inode},
    global_state::GlobalState,
    scheduler::{Context, switch_to_scheduler_context},
    syscall::stdout,
    traps::{self, set_up_supervisor_to_user_mode_transition, user_trap},
    vm::{self, drop_pages, kernel_stack_address, map, map_trampoline},
};
use alloc::{
    alloc::dealloc,
    boxed::Box,
    collections::btree_map::BTreeMap,
    format,
    rc::Rc,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use anyhow::{Result, bail};
use core::{
    arch::{asm, global_asm},
    cell::LazyCell,
    fmt::Debug,
    mem::{self, transmute},
    ptr::{self, null_mut, write_volatile},
};
use elf::{ElfBytes, endian::NativeEndian};
use lock_api::MutexGuard;

pub enum ProcessState {
    Ready {
        cwd: Rc<MemoryINode>,
    },
    Running {
        cwd: Rc<MemoryINode>,
    },
    Terminated {
        return_value: core::result::Result<isize, Box<dyn Debug>>,
    },
    NotUsed,
    Sleeping {
        cwd: Rc<MemoryINode>,
        sleep_on: usize,
    },
}

pub struct Process {
    pub id: usize,
    pub name: Box<str>,

    /// CAUTION: Holds the bottom of the stack
    kernel_stack: usize,
    pub process_state: ProcessState,
    pub context: Context,
    parent: Option<Weak<Process>>,
    pub children: Option<Arc<Mutex<Process>>>,
    pub page_table: usize,
    pub code: usize,
    pub trapframe: *mut TrapFrame,
    pub fds: Vec<usize>,
    pub size: usize,
    pub argv_addr: Vec<usize>,
    pub global_state: &'static GlobalState,
    pub heap_end: usize,
    pub brk: usize,
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("Process: {}", self.id))
    }
}

impl Process {
    pub fn default(id: usize, context: &'static GlobalState) -> Self {
        Process {
            id,
            name: "".into(),
            kernel_stack: 0,
            process_state: ProcessState::NotUsed,
            context: Context::default(),
            parent: None,
            children: None,
            page_table: 0,
            code: 0,
            trapframe: null_mut(),
            fds: Vec::new(),
            size: 0,
            argv_addr: Vec::new(),
            global_state: context,
            heap_end: 0,
            brk: 0,
        }
    }

    // pub fn clone(&'static mut self) -> Result<Arc<Spinlock<Process>>> {
    //     let process = assign_process(self.global_state)?;

    //     {
    //         let mut process = process.lock();

    //         process.context = self.context.clone();

    //         process.fds = self.fds.clone();
    //         process.kernel_stack = kernel_stack_address(process.id);
    //         process.context.sp = process.kernel_stack + PAGE_SIZE;
    //         process.context.ra = prepare_first_time_execution as usize;
    //         process.size = self.size;

    //         unsafe {
    //             *process.trapframe = (*self.trapframe).clone();
    //             (*process.trapframe).a0 = 0;
    //             (*self.trapframe).a0 = process.id;

    //             (*process.trapframe).page_table = process.page_table;
    //             (*process.trapframe).kernel_stack = process.kernel_stack;
    //             (*process.trapframe).satp = Sv48 | ((*process.trapframe).page_table >> 12);
    //         }

    //         map(
    //             self.global_state,
    //             process.page_table,
    //             TRAPFRAME,
    //             process.trapframe.addr(),
    //             PAGE_SIZE,
    //             READ_WRITE,
    //         )?;

    //         process.parent = Some();

    //         process.name = self.name.clone();

    //         if let ProcessState::Running { cwd } = &self.process_state {
    //             process.process_state = ProcessState::Ready { cwd: cwd.clone() };
    //         }

    //         vm::copy(
    //             self.global_state,
    //             self.page_table,
    //             process.page_table,
    //             self.size,
    //         )?;

    //         map_trampoline(
    //             self.global_state,
    //             process.page_table,
    //             TRAMPOLINE,
    //             unsafe { TRAMPOLINE_CODE_ADDRESS },
    //             PAGE_SIZE,
    //             READ_EXECUTE,
    //         )?;
    //     }
    //     Ok(process)
    // }
}

pub fn yield_cpu(state: &GlobalState) {
    let locked_process = state
        .get_current_process()
        .expect("YIELD FAILED - NO CURRENT PROCESS");

    let mut process = locked_process.lock();
    if let ProcessState::Running { cwd } = &process.process_state {
        process.process_state = ProcessState::Ready { cwd: cwd.clone() }
    }
    drop(process);
    switch_to_scheduler_context(state);
}

pub fn are_interrupts_enabled() -> bool {
    unsafe {
        let sstatus: usize;
        asm!("csrr {}, sstatus", out(reg) sstatus);
        (sstatus & (1 << 1)) != 0
    }
}
pub fn assign_process(state: &'static GlobalState) -> Result<Arc<Mutex<Process>>> {
    static PID: spin::Mutex<usize> = spin::Mutex::new(0);

    let page_table = state.allocate(PAGE_SIZE).unwrap();
    let trapframe = state.allocate(PAGE_SIZE).unwrap() as *mut TrapFrame;

    let mut pid = PID.lock();
    *pid += 1;

    let locked_process = Arc::new(Mutex::new(Process::default(*pid - 1, state)));
    state.add_process(*pid, locked_process.clone());

    let mut process = locked_process.lock();

    process.process_state = ProcessState::Ready {
        cwd: read_inode(ROOT_INODE, &DEVICE),
    };
    process.page_table = page_table;

    process.trapframe = trapframe;

    unsafe {
        *trapframe = TrapFrame::default();
        (*trapframe).page_table = page_table;
        (*trapframe).satp = Sv48 | page_table >> 12;
        (*trapframe).kernel_page_table = Sv48 | (KERNEL_PAGE_TABLE >> 12);
        (*trapframe).user_trap_address = user_trap as usize;
    }

    drop(process);

    return Ok(locked_process.clone());
}

pub fn map_code_pages(
    state: &GlobalState,
    page_table: usize,
    code_pa: usize,
    code_va: usize,
    num_code_pages: usize,
    read: bool,
    write: bool,
    execute: bool,
) {
    if code_va == TRAMPOLINE {
        panic!("PROCESS CREATION FAILED - CODE SEGMENT CANNOT BE MAPPED TO TRAMPOLINE ADDRESS");
    } else if code_va == TRAPFRAME {
        panic!("PROCESS CREATION FAILED - CODE SEGMENT CANNOT BE MAPPED TO TRAPFRAME ADDRESS");
    }
    state
        .map(
            page_table,
            code_va,
            code_pa,
            num_code_pages * PAGE_SIZE,
            read,
            write,
            execute,
            true,
        )
        .expect("PROCESS CREATION FAILED - ERROR WHILE MAPPING CODE PAGES");
}

pub fn map_other_pages(
    state: &GlobalState,
    page_table: usize,
    final_code: usize,
    process: &mut MutexGuard<sync::RawMutex<PAGE_TABLE_ENTRY, ARCH>, Process>,
) -> Result<()> {
    if (TRAPFRAME - final_code) < (10 * PAGE_SIZE) {
        panic!("PROCESS CREATION FAILED - NOT ENOUGH SPACE FOR STACK AND HEAP");
    }

    let stack = state.allocate(STACK_PAGES * PAGE_SIZE).unwrap();

    state.map(
        page_table,
        STACK_START,
        stack,
        STACK_PAGES * PAGE_SIZE,
        true,
        true,
        false,
        true,
    )?;

    process.kernel_stack = kernel_stack_address(process.id);

    state.map(
        page_table,
        TRAMPOLINE,
        unsafe { TRAMPOLINE_CODE_ADDRESS },
        PAGE_SIZE,
        true,
        false,
        true,
        false,
    )?;

    let trapframe = process.trapframe;

    state.map(
        page_table,
        TRAPFRAME,
        trapframe.addr(),
        PAGE_SIZE,
        true,
        true,
        false,
        false,
    )?;

    unsafe {
        (*trapframe).kernel_page_table = Sv48 | (KERNEL_PAGE_TABLE >> 12);
        (*trapframe).kernel_stack = process.kernel_stack;
        (*trapframe).sp = TRAMPOLINE - 2 * PAGE_SIZE;
    }

    Ok(())
}

pub fn start_init(state: &'static GlobalState) {
    let mut page = 0;
    let mut max_code_page_end_va = 0;

    let process = assign_process(state).expect("INIT FAILED - FAILED TO ASSIGN PROCESS");
    let mut process = process.lock();
    process.name = "init".into();

    let elf_data: ElfBytes<NativeEndian> =
        elf::ElfBytes::minimal_parse(INIT).expect("INIT FAILED - ELF ERROR");

    let program_headers = elf_data
        .segments()
        .expect("INIT FAILED - NO SEGMENTS")
        .iter();

    for header in program_headers {
        let file_size = header.p_filesz as usize;
        let mem_size = header.p_memsz as usize;

        let num_pages = (mem_size + PAGE_SIZE - 1) / PAGE_SIZE;

        if num_pages == 0 {
            continue;
        }

        page = state
            .allocate((num_pages * PAGE_SIZE).next_power_of_two())
            .unwrap();

        let offset = header.p_offset as usize;

        let va = header.p_vaddr as usize;
        let flags = header.p_flags;

        if va + num_pages * PAGE_SIZE > max_code_page_end_va {
            max_code_page_end_va = va + num_pages * PAGE_SIZE;
        }

        let mut read = false;
        let mut write = false;
        let mut execute = false;

        if flags & elf::abi::PF_R != 0 {
            read = true;
        }

        if flags & elf::abi::PF_W != 0 {
            write = true;
        }

        if flags & elf::abi::PF_X != 0 {
            execute = true;
        }

        let loadable = &INIT[offset..offset + file_size];

        map_code_pages(
            state,
            process.page_table,
            page,
            va,
            num_pages,
            read,
            write,
            execute,
        );
        process.size += num_pages * PAGE_SIZE;

        unsafe {
            ptr::copy_nonoverlapping(loadable.as_ptr(), page as *mut u8, file_size);
        }
    }

    if page == 0 {
        panic!("PANIC: INIT FAILED - ELF CONTAINS NO LOADABLE SEGMENT");
    }

    map_other_pages(
        state,
        process.page_table,
        max_code_page_end_va,
        &mut process,
    )
    .expect("INIT FAILED - ERROR WHILE MAPPING PAGES");

    let trapframe = process.trapframe;

    unsafe {
        (*trapframe).sepc = elf_data.ehdr.e_entry as usize;
    }
    process.context.ra = prepare_first_time_execution as fn() as usize;

    unsafe {
        process.context.sp = (*trapframe).kernel_stack + 4 * PAGE_SIZE;
        process.brk = max_code_page_end_va;
        process.heap_end = max_code_page_end_va;
    }
}

/// This function is called when a process has to be executed for the first time.
pub fn prepare_first_time_execution() {
    let state = traps::get_global_state();
    let locked_process = state.get_current_process().expect("NO CURRENT PROCESS");
    let trapframe;

    let process = locked_process.lock();

    trapframe = process.trapframe;
    drop(process);
    set_up_supervisor_to_user_mode_transition(state)
        .expect("INIT FAILED - CONTEXT NONE WHILE RETURNING TO USER MODE");

    unsafe {
        let return_to_user_mode_ptr: fn(usize) -> ! = transmute(TRAMPOLINE + TRAMPOLINE_OFFSET);
        return_to_user_mode_ptr(trapframe.addr());
    }
}

impl Process {
    pub fn sleep(&mut self, sleep_on: usize) {
        match &self.process_state {
            ProcessState::Running { cwd } | ProcessState::Ready { cwd } => {
                self.process_state = ProcessState::Sleeping {
                    cwd: cwd.clone(),
                    sleep_on,
                }
            }
            _ => (),
        }

        switch_to_scheduler_context(self.global_state);
    }
}

pub fn wake_up(state: &GlobalState, sleep_on_arg: usize) {
    if let Some((pid, cwd)) = state.find_sleeping_process(sleep_on_arg) {
        let process = state.get_process(pid).unwrap();
        let mut process = process.lock();
        process.process_state = ProcessState::Ready { cwd }
    }
}
