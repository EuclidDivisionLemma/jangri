use crate::{
    ARCH, Mutex, PAGE_TABLE_ENTRY, TrapFrame,
    constants::{KERNEL_PAGE_TABLE, Sv48, TRAMPOLINE_CODE_ADDRESS, TRAMPOLINE_OFFSET},
    global_state::GlobalState,
    scheduler::{Context, switch_to_scheduler_context},
    traps::{self, set_up_supervisor_to_user_mode_transition, user_trap},
    vm::kernel_stack_address,
};
use alloc::{
    boxed::Box,
    format,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{
    fmt::Debug,
    mem::{self, transmute},
    ptr::{self, null_mut, write_bytes},
    slice,
};
use elf::{
    ElfBytes,
    abi::{PF_R, PF_W, PF_X, PT_LOAD, SHT_NOBITS},
    endian::NativeEndian,
};
use hal::{
    Hal,
    constants::{KUCOM_PAGE, PAGE_SIZE, STACK_PAGES, TRAMPOLINE, TRAPFRAME},
    vm::align_to_page_size,
};
use hal::{
    constants::STACK_START,
    error::{Error, Result},
};
use lock_api::MutexGuard;

#[derive(Debug)]
pub enum ProcessState {
    Ready,
    Running,
    Terminated {
        #[allow(unused)]
        return_value: core::result::Result<usize, Box<dyn Debug>>,
    },
    NotUsed,
    Waiting {
        waiting_for: usize,
    },
}

pub struct Process {
    pub id: usize,
    pub name: Box<str>,

    /// CAUTION: Holds the bottom of the stack
    kernel_stack: usize,
    pub process_state: ProcessState,
    pub context: Context,
    pub page_table: usize,
    pub trapframe: *mut TrapFrame,
    pub size: usize,
    pub global_state: &'static GlobalState,
    pub heap_start: usize,
    pub heap_end: usize,
    pub currently_unmapped_start: usize,
    pub parent: Option<Weak<Mutex<Process>>>,
    pub children: Vec<Arc<Mutex<Process>>>,
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
            page_table: 0,
            trapframe: null_mut(),
            size: 0,
            global_state: context,
            heap_start: 0,
            heap_end: 0,
            currently_unmapped_start: 0,
            parent: None,
            children: Vec::new(),
        }
    }
}

pub fn yield_cpu(state: &GlobalState) {
    {
        let locked_process = state
            .get_current_process()
            .expect("YIELD FAILED - NO CURRENT PROCESS");

        let mut process = locked_process.lock();
        if let ProcessState::Running = &process.process_state {
            process.process_state = ProcessState::Ready;
        }
    }
    switch_to_scheduler_context(state);
}

pub fn assign_process(
    state: &'static GlobalState,
    name: &str,
    image: Vec<u8>,
) -> Result<Arc<Mutex<Process>>> {
    static PID: spin::Mutex<usize> = spin::Mutex::new(0);

    let page_table = state.allocate(PAGE_SIZE).unwrap();
    let trapframe = state.allocate(PAGE_SIZE).unwrap() as *mut TrapFrame;

    let mut pid = PID.lock();
    *pid += 1;

    let locked_process = Arc::new(Mutex::new(Process::default(*pid, state)));
    state.add_process(*pid, locked_process.clone());

    let mut process = locked_process.lock();
    process.name = name.into();
    process
        .context
        .set_return_address(prepare_first_time_execution as fn() as usize);

    process.process_state = ProcessState::Ready;
    process.page_table = page_table;

    process.trapframe = trapframe;

    #[cfg(target_arch = "riscv64")]
    unsafe {
        *trapframe = TrapFrame::default();
        (*trapframe).page_table = page_table;
        (*trapframe).satp = Sv48 | page_table >> 12;
        (*trapframe).kernel_page_table = Sv48 | (KERNEL_PAGE_TABLE >> 12);
        (*trapframe).user_trap_address = user_trap as fn() as usize;
    }

    let elf_bytes = elf::ElfBytes::<NativeEndian>::minimal_parse(image.as_slice())
        .map_err(|_| Error::ELFError)?;

    for segment in elf_bytes.segments().ok_or(Error::ELFError)? {
        if segment.p_type == PT_LOAD {
            let code_page_start = align_to_page_size(segment.p_vaddr as usize);
            let memsz = align_to_page_size(segment.p_memsz as usize);
            let mut pt = state.allocate(memsz)?;

            state.map(
                page_table,
                code_page_start,
                pt,
                align_to_page_size(segment.p_memsz as usize), // results in space waste
                if segment.p_flags & PF_R != 0 {
                    true
                } else {
                    false
                },
                if segment.p_flags & PF_W != 0 {
                    true
                } else {
                    false
                },
                if segment.p_flags & PF_X != 0 {
                    true
                } else {
                    false
                },
                true,
            )?;
            let pt = unsafe { slice::from_raw_parts_mut(pt as *mut u8, segment.p_filesz as usize) };
            pt.copy_from_slice(
                &image[segment.p_offset as usize..(segment.p_offset + segment.p_filesz) as usize],
            );

            if process.heap_start <= code_page_start {
                process.heap_start = process
                    .heap_start
                    .checked_add(code_page_start + PAGE_SIZE)
                    .unwrap();
            }
        }

        process.heap_end = process.heap_start;
        process.currently_unmapped_start = process.heap_end;
    }

    hal::interrupts::TrapFrame::set_entry_point(trapframe, elf_bytes.ehdr.e_entry as usize);
    map_other_pages(state, process.page_table, &mut process)
        .expect("PROCESS CREATION FAILED - ERROR WHILE MAPPING PAGES");

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
    process: &mut MutexGuard<sync::RawMutex<PAGE_TABLE_ENTRY, ARCH>, Process>,
) -> Result<()> {
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
    let ks = process.kernel_stack;
    process.context.set_sp(ks + 4 * PAGE_SIZE);

    hal::interrupts::TrapFrame::set_sp(process.trapframe, STACK_START + STACK_PAGES * PAGE_SIZE);

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

    let error_page = state.allocate(PAGE_SIZE)?;

    state.map(
        page_table, KUCOM_PAGE, error_page, PAGE_SIZE, true, true, false, true,
    )?;

    unsafe {
        (*trapframe).kernel_page_table = Sv48 | (KERNEL_PAGE_TABLE >> 12);
        (*trapframe).kernel_stack = process.kernel_stack;
    }

    Ok(())
}

/// This function is called when a process has to be executed for the first time.
pub fn prepare_first_time_execution() {
    let state = traps::get_global_state();
    let trapframe: *mut TrapFrame = {
        let process: Arc<Mutex<Process>> = state.get_current_process().expect("NO CURRENT PROCESS");
        let process = process.lock();
        process.trapframe
    };
    set_up_supervisor_to_user_mode_transition(state)
        .expect("INIT FAILED - CONTEXT NONE WHILE RETURNING TO USER MODE");

    unsafe {
        let return_to_user_mode_ptr: fn(usize) -> ! = transmute(TRAMPOLINE + TRAMPOLINE_OFFSET);
        return_to_user_mode_ptr(trapframe.addr());
    }
}

impl Process {
    pub fn wait(&mut self, wait_for: usize) {
        self.process_state = ProcessState::Waiting {
            waiting_for: wait_for,
        };

        switch_to_scheduler_context(self.global_state);
    }
}
