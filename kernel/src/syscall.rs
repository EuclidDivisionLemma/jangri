use core::{arch::asm, ffi::c_int, mem, ptr::write_volatile, slice};

use alloc::{boxed::Box, sync::Arc};
use hal::{
    constants::{ERROR_PAGE, PAGE_SIZE, STACK_GUARD},
    error::{Error, Result},
    interrupts::{InterruptHandling, SyscallArgs, TrapFrame},
};
use janglib::{Syscall, get_error, memory::UserMemorySlice};
use ringbuffer::RingBuffer;
use riscv_arch::uart::{self, INPUT_BUFFER};

use crate::{
    ARCH, Mutex,
    global_state::GlobalState,
    process::{self, Process, ProcessState, assign_process, prepare_first_time_execution},
    scheduler::switch_to_scheduler_context,
};

use hal::vm::align_to_page_size;

pub fn stdout<'a>(text: &'a str) {
    let chars = text.as_bytes();
    unsafe {
        for char in chars {
            asm!("li a7, 0x4442434E",
            "li a6, 2",
            "mv a0, {}",
            "ecall",
            in(reg) *char as i64);
        }
    }
}

pub fn handle(state: &'static GlobalState) {
    if let Some(locked_process) = state.get_current_process() {
        let trapframe;

        let process = locked_process.lock();
        let page_table = process.page_table;
        trapframe = process.trapframe;
        drop(process);

        let args = ARCH::handle_syscall(trapframe);

        let mut error_page = UserMemorySlice::<true, _>::new(
            ERROR_PAGE,
            page_table,
            PAGE_SIZE,
            |va: usize, page_table: usize| state.va2pa(page_table, va),
        );

        let syscall = if let Ok(syscall) = Syscall::try_from(args.0) {
            syscall
        } else {
            TrapFrame::set_success_indicator(trapframe, 1);
            let e = unsafe {
                mem::transmute::<_, [u8; size_of::<Error>()]>(Error::InvalidSyscallNo(args.0))
            };
            error_page.write(unsafe { e.as_slice() });

            unsafe {
                state.enable_interrupts();
            }
            return;
        };

        let result = || -> Result<usize> {
            match syscall {
                Syscall::WantMemory => {
                    let size = args.1;
                    want_memory(state, size)
                }
                Syscall::Write => {
                    let start = args.1 as *const u8;
                    let len = args.2;
                    let slice = janglib::memory::UserMemorySlice::<false, _>::new(
                        start.addr(),
                        page_table,
                        len,
                        |va, page_table| state.va2pa(page_table, va),
                    );
                    uart::console_write(str::from_utf8(slice.read()).unwrap());
                    Ok(len)
                }
                Syscall::ReadChar => {
                    let mut ch = uart::read_char();

                    while let None = ch {
                        ch = uart::read_char();
                    }
                    Ok(ch.unwrap() as usize)
                }
                Syscall::Exit => exit(state, args),
                Syscall::Spawn => {
                    let start = args.1;
                    let len = args.2;
                    let image = UserMemorySlice::<false, _>::new(
                        start,
                        page_table,
                        len,
                        |start, page_table| state.va2pa(page_table, start),
                    )
                    .read_to_vec();
                    let process = assign_process(state, "", image)?;
                    let process = process.lock();

                    Ok(0)
                }
            }
        };
        let result = result();

        match result {
            Ok(v) => {
                TrapFrame::set_success_indicator(trapframe, 0);
                TrapFrame::set_return_value(trapframe, v);
            }
            Err(e) => {
                let e = unsafe { mem::transmute::<_, [u8; size_of::<Error>()]>(e) };
                TrapFrame::set_success_indicator(trapframe, 1);
                error_page.write(unsafe { e.as_slice() });
            }
        }
    } else {
        panic!("SYSCALLd, BUT NO RUNNING PROCESS")
    }
}

pub fn want_memory(state: &GlobalState, size: usize) -> Result<usize> {
    let process: Arc<Mutex<Process>> = state.get_current_process().unwrap();
    let mut process = process.lock();
    let va = process.heap_end;

    assert!(size % PAGE_SIZE == 0);

    let num_pages = size / PAGE_SIZE;

    if va + num_pages * PAGE_SIZE >= STACK_GUARD {
        return Err(Error::MemoryNotAvailable);
    }

    let pa = state.allocate(size)?;
    state.map(
        process.page_table,
        va,
        pa,
        num_pages * PAGE_SIZE,
        true,
        true,
        false,
        true,
    )?;

    process.heap_end += num_pages * PAGE_SIZE;

    Ok(process.heap_end - num_pages * PAGE_SIZE)
}

fn exit(state: &GlobalState, args: SyscallArgs) -> ! {
    let current_process = state.get_current_process().unwrap();
    let mut current_process = current_process.lock();
    current_process.process_state = ProcessState::Terminated {
        return_value: if args.1 == 0 {
            Ok(args.2)
        } else {
            let trapframe = current_process.trapframe;
            unsafe { Err(Box::new(unsafe { get_error() })) }
        },
    };
    state
        .cleanup_page_table(current_process.page_table)
        .unwrap();
    drop(current_process);
    switch_to_scheduler_context(state);
}
