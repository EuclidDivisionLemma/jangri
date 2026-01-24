use core::{
    arch::asm,
    ffi::{CStr, c_int},
    ptr::slice_from_raw_parts_mut,
};

use hal::interrupts::{InterruptHandling, Syscall, SyscallArgs, TrapFrame};

use crate::{
    ARCH,
    file::{FILES, allocate_file},
    global_state::GlobalState,
    pipe::allocate_pipe,
    process::ProcessState,
    syscall::{
        fs::{chdir, mkdir},
        io::Error,
        process::exit,
    },
    vm::{self, translate_virtual_address},
};

pub mod fs;
pub mod io;
pub mod process;

pub const SYSCALLS: [(Syscall, fn(&GlobalState, SyscallArgs) -> usize); 11] = [
    (Syscall::Open, io::open),
    (Syscall::Read, io::read),
    (Syscall::Write, io::write),
    (Syscall::Close, io::close),
    (Syscall::Lseek, io::lseek),
    (Syscall::Pipe, pipe),
    (Syscall::Sbrk, sbrk),
    (Syscall::Exit, exit),
    (Syscall::Dup2, dup2),
    (Syscall::Chdir, chdir),
    (Syscall::Mkdir, mkdir),
];

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
        trapframe = process.trapframe;
        drop(process);

        let args;
        unsafe {
            args = ARCH::handle_syscall(trapframe);

            // sepc holds the program counter value at the point of trap
            // But when the trap is due to a system call, we need to execute the next instruction

            state.enable_interrupts();
        }

        for (no, handler) in SYSCALLS {
            if args.0 == no as usize {
                let return_value = handler(state, args);
                let process = locked_process.lock();
                let trapframe = process.trapframe;

                TrapFrame::set_return_value_after_syscall(trapframe, return_value);

                return;
            }
        }
        unsafe {
            (*trapframe).a0 = -1isize as usize;
        }
    } else {
        panic!("SYSCALLd, BUT NO RUNNING PROCESS")
    }
}

pub fn pipe(state: &GlobalState, args: SyscallArgs) -> usize {
    let current_process = state.get_current_process().unwrap();
    let current_process = current_process.lock();

    let state = current_process.global_state;

    let writer = allocate_file();
    let reader = allocate_file();

    let _ = allocate_pipe(state, &reader, &writer);

    let fds = unsafe {
        &mut *slice_from_raw_parts_mut(
            translate_virtual_address(state, current_process.page_table, args.1).unwrap()
                as *mut c_int,
            2,
        )
    };

    fds[0] = *reader.fd.borrow() as c_int;
    fds[1] = *writer.fd.borrow() as c_int;

    0
}

pub fn sbrk(state: &GlobalState, args: SyscallArgs) -> usize {
    enum Error {
        ENOMEM = 12,
    }
    let increment = args.1 as isize;

    let current_process = state.get_current_process().unwrap();
    let current_process = current_process.lock();

    if increment == 0 {
        current_process.brk
    } else if increment < 0 {
        panic!("NEGATIVE INCREMENT NOT ALLOWED YET!")
    } else {
        match vm::allocate_heap(state, increment, current_process) {
            Ok(old_brk) => old_brk,
            Err(e)
                if matches!(
                    e.downcast_ref().unwrap(),
                    crate::error::Error::InvalidHeapSize
                ) =>
            {
                -(Error::ENOMEM as isize) as usize
            }
            Err(e) => panic!("HEAP ALLOCATION FAILED: {}", e),
        }
    }
}

pub fn dup2(_: &GlobalState, args: SyscallArgs) -> usize {
    let fd1 = args.1;
    let fd2 = args.2;

    if fd1 == fd2 {
        return fd2;
    }

    if let Some(_) = unsafe { FILES.get(&fd2) } {
        unsafe {
            FILES.remove(&fd2);
        }
    }

    match unsafe { FILES.get(&fd1) } {
        Some(file) => {
            let new_file = file.clone();
            *new_file.fd.borrow_mut() = fd2;

            unsafe {
                FILES.insert(fd2, new_file);
            }

            fd2
        }
        None => -Error::EBADF as usize,
    }
}
