use core::{arch::asm, ffi::c_int, ptr::slice_from_raw_parts_mut};

use alloc::format;

use crate::{
    file::{FILES, allocate_file},
    pipe::allocate_pipe,
    process::CURRENT_PROCESS,
    syscall::process::{exit, fork},
    traps::TrapFrame,
    vm::{self, translate_virtual_address},
};

pub mod io;
pub mod process;

pub const SYSCALLS: [(Syscall, fn(&TrapFrame) -> usize); 9] = [
    (Syscall::Open, io::open),
    (Syscall::Read, io::read),
    (Syscall::Write, io::write),
    (Syscall::Close, io::close),
    (Syscall::Lseek, io::lseek),
    (Syscall::Pipe, pipe),
    (Syscall::Sbrk, sbrk),
    (Syscall::Exit, exit),
    (Syscall::Fork, fork),
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
}

pub fn stdout<'a>(text: &'a str) {
    let chars = text.as_bytes();
    unsafe {
        for char in chars {
            asm!("li a7, 0x4442434E",
            "li a6, 2",
            "mv a0, {}",
            "ecall",
            in(reg) *char);
        }
    }
}

pub fn handle() {
    let syscall_no: usize;

    if let Some(process) = unsafe { &mut CURRENT_PROCESS } {
        let trapframe = process
            .trapframe
            .as_mut()
            .expect("TRAPFRAME NONE WHILE HANDLING TRAP");
        syscall_no = trapframe.a7;

        // sepc holds the program counter value at the point of trap
        // But when the trap is due to a system call, we need to execute the next instruction
        trapframe.sepc += 4;

        unsafe {
            riscv::interrupt::supervisor::enable();
        }

        for (no, handler) in SYSCALLS {
            if syscall_no == no as usize {
                trapframe.a0 = handler(&trapframe);
                return;
            }
        }
        trapframe.a0 = -1isize as usize;
    } else {
        panic!("SYSCALLd, BUT NO RUNNING PROCESS")
    }
}

pub fn pipe(trapframe: &TrapFrame) -> usize {
    let writer = allocate_file();
    let reader = allocate_file();

    let _ = allocate_pipe(&reader, &writer);

    let fds = unsafe {
        &mut *slice_from_raw_parts_mut(
            translate_virtual_address(trapframe.page_table, trapframe.a0).unwrap() as *mut c_int,
            2,
        )
    };

    fds[0] = reader.fd as c_int;
    fds[1] = writer.fd as c_int;

    0
}

pub fn sbrk(trapframe: &TrapFrame) -> usize {
    enum Error {
        ENOMEM = 12,
    }
    let increment = trapframe.a0 as isize;

    if increment == 0 {
        trapframe.brk.get()
    } else if increment < 0 {
        panic!("NEGATIVE INCREMENT NOT ALLOWED YET!")
    } else {
        match vm::allocate_heap(increment, trapframe) {
            Ok(old_brk) => old_brk,
            Err(e) if e == crate::error::Error::InvalidHeapSize => {
                -(Error::ENOMEM as isize) as usize
            }
            Err(e) => panic!("HEAP ALLOCATION FAILED: {}", e),
        }
    }
}
