use core::{arch::asm, ffi::c_int, ptr::write_volatile, slice};

use alloc::boxed::Box;
use hal::{
    constants::PAGE_SIZE,
    error::{Error, Result},
    interrupts::{InterruptHandling, SyscallArgs, TrapFrame},
};
use janglib::Syscall;
use ringbuffer::RingBuffer;
use riscv_arch::uart::{self, INPUT_BUFFER};

use crate::{
    ARCH,
    constants::TRAMPOLINE,
    global_state::GlobalState,
    process::{self, ProcessState},
    scheduler::switch_to_scheduler_context,
};

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

pub fn handle(state: &GlobalState) {
    if let Some(locked_process) = state.get_current_process() {
        let trapframe;

        let process = locked_process.lock();
        let page_table = process.page_table;
        trapframe = process.trapframe;
        drop(process);

        let args = ARCH::handle_syscall(trapframe);

        let syscall = if let Ok(syscall) = Syscall::try_from(args.0) {
            syscall
        } else {
            TrapFrame::set_success_indicator(trapframe, 1);
            TrapFrame::set_error(trapframe, Error::InvalidSyscallNo(args.0));
            unsafe {
                state.enable_interrupts();
            }
            return;
        };

        let result = match syscall {
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
        };

        match result {
            Ok(v) => {
                TrapFrame::set_success_indicator(trapframe, 0);
                TrapFrame::set_return_value(trapframe, v);
            }
            Err(e) => {
                TrapFrame::set_success_indicator(trapframe, 1);
                TrapFrame::set_error(trapframe, e);
            }
        }
    } else {
        panic!("SYSCALLd, BUT NO RUNNING PROCESS")
    }
}

pub fn want_memory(state: &GlobalState, increment: usize) -> Result<usize> {
    let current_process = state.get_current_process().unwrap();
    let mut current_process = current_process.lock();

    if increment + current_process.brk < current_process.heap_end {
        let old = current_process.brk;
        current_process.brk += increment;
        return Ok(old);
    }

    assert!(current_process.brk == current_process.heap_end);

    if increment + current_process.brk < TRAMPOLINE - 11 * PAGE_SIZE {
        let num_pages = (increment + PAGE_SIZE - 1) / PAGE_SIZE;

        if let Ok(pa) = state.allocate(num_pages * PAGE_SIZE) {
            if let Ok(()) = state.map(
                current_process.page_table,
                current_process.brk,
                pa,
                num_pages * PAGE_SIZE,
                true,
                true,
                false,
                true,
            ) {
                let old = current_process.brk;
                current_process.brk += increment;
                current_process.heap_end += num_pages * PAGE_SIZE;
                return Ok(old);
            } else {
                state.deallocate(pa, num_pages * PAGE_SIZE);
            }
        }
    }

    Err(Error::MemoryNotAvailable)
}

fn exit(state: &GlobalState, args: SyscallArgs) -> ! {
    let current_process = state.get_current_process().unwrap();
    let mut current_process = current_process.lock();
    current_process.process_state = ProcessState::Terminated {
        return_value: if args.1 == 0 {
            Ok(args.2)
        } else {
            let trapframe = current_process.trapframe;
            unsafe { Err(Box::new((*trapframe).error.unwrap())) }
        },
    };
    state
        .cleanup_page_table(current_process.page_table)
        .unwrap();
    drop(current_process);
    switch_to_scheduler_context(state);
}
