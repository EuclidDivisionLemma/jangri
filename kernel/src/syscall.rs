use core::{
    arch::asm,
    ffi::{CStr, c_int},
    ptr::slice_from_raw_parts_mut,
};

use alloc::{format, vec::Vec};
use sync::Lock;

use crate::{
    file::{self, FILES, allocate_file, exists, traverse_path},
    global_state::GlobalState,
    pipe::allocate_pipe,
    process::ProcessState,
    syscall::{
        fs::{chdir, mkdir},
        io::Error,
        process::exit,
    },
    traps::TrapFrame,
    vm::{self, translate_virtual_address},
};

pub mod fs;
pub mod io;
pub mod process;

pub const SYSCALLS: [(Syscall, fn(&TrapFrame) -> usize); 11] = [
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

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Syscall {
    Open = 100,
    Read = 200,
    Write = 300,
    Close = 400,
    Lseek = 500,
    Pipe = 600,
    Sbrk = 700,
    Exit = 800,
    Fork = 900,
    Wait = 1000,
    Dup2 = 1200,
    Chdir = 1300,
    Mkdir = 1400,
}

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

pub fn handle() {
    let syscall_no: usize;

    let state = GlobalState::get();

    if let Some(locked_process) = state.get_current_process() {
        let trapframe;

        let process = locked_process.lock();
        trapframe = process.trapframe;
        drop(process);

        unsafe {
            syscall_no = (*trapframe).a7;

            // sepc holds the program counter value at the point of trap
            // But when the trap is due to a system call, we need to execute the next instruction
            (*trapframe).sepc += 4;

            riscv::interrupt::supervisor::enable();
        }

        for (no, handler) in SYSCALLS {
            if syscall_no == no as usize {
                unsafe {
                    let return_value = handler(&*trapframe);
                    let process = locked_process.lock();
                    let trapframe = process.trapframe;

                    (*trapframe).a0 = return_value;
                }
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

pub fn pipe(trapframe: &TrapFrame) -> usize {
    let state = GlobalState::get();

    let current_process = state.get_current_process().unwrap();
    let current_process = current_process.lock();

    let state = current_process.global_state;

    let writer = allocate_file();
    let reader = allocate_file();

    let _ = allocate_pipe(&reader, &writer);

    let fds = unsafe {
        &mut *slice_from_raw_parts_mut(
            translate_virtual_address(state, trapframe.page_table, trapframe.a0).unwrap()
                as *mut c_int,
            2,
        )
    };

    fds[0] = *reader.fd.borrow() as c_int;
    fds[1] = *writer.fd.borrow() as c_int;

    0
}

pub fn sbrk(trapframe: &TrapFrame) -> usize {
    let state = GlobalState::get();

    enum Error {
        ENOMEM = 12,
    }
    let increment = trapframe.a0 as isize;

    if increment == 0 {
        trapframe.brk.get()
    } else if increment < 0 {
        panic!("NEGATIVE INCREMENT NOT ALLOWED YET!")
    } else {
        match vm::allocate_heap(state, increment, trapframe) {
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

pub fn dup2(trapframe: &TrapFrame) -> usize {
    let fd1 = trapframe.a0;
    let fd2 = trapframe.a1;

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
