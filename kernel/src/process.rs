use crate::{
    constants::{MAXIMUM_PROCESS, PAGE_SIZE, TRAMPOLINE},
    traps::TrapFrame,
};

static mut PROCESSES: [Process; MAXIMUM_PROCESS] = [Process::default(); MAXIMUM_PROCESS];

pub fn intialise_processes() {
    unsafe {
        for i in 0..MAXIMUM_PROCESS {
            PROCESSES[i].id = i;
            PROCESSES[i].kernel_stack = TRAMPOLINE - (i + 1) * 2 * PAGE_SIZE;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
    NotUsed,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Process<'a> {
    id: usize,
    name: &'a str,
    kernel_stack: usize,
    state: ProcessState,
    context: Option<TrapFrame>,
}

impl<'a> Process<'a> {
    const fn default() -> Self {
        Process {
            id: 0,
            name: "",
            kernel_stack: 0,
            state: ProcessState::NotUsed,
            context: None,
        }
    }
}
