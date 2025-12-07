use core::{arch::asm, slice, str};

use crate::process::CURRENT_PROCESS;

pub enum SyscallErrors {
    StringInvalid = 2,
    UnknownSyscall = 1,
}

pub enum SyscallNumbers {
    Stdout = 0,
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

    unsafe {
        asm!("mv {}, a7", out(reg) syscall_no);
    }

    if let Some(process) = unsafe { &mut CURRENT_PROCESS } {
        let trapframe = process
            .trapframe
            .as_mut()
            .expect("TRAPFRAME NONE WHILE HANDLING TRAP");
        // sepc holds the program counter value at the point of trap
        // But when the trap is due to a system call, we need to execute the next instruction
        trapframe.sepc += 4;

        if syscall_no == SyscallNumbers::Stdout as usize {
            let length: usize;
            let ptr: usize;

            unsafe {
                asm!("mv {}, a0", out(reg) length);
                asm!("mv {}, a1", out(reg) ptr);
            }

            let s = match unsafe { str::from_utf8(slice::from_raw_parts(ptr as *const u8, length)) }
            {
                Ok(s) => s,
                Err(_) => {
                    trapframe.a0 = SyscallErrors::StringInvalid as usize;
                    return;
                }
            };

            stdout(s);
            trapframe.a0 = 0;
        } else {
            trapframe.a0 = SyscallErrors::UnknownSyscall as usize;
        }
    } else {
        panic!("SYSCALLd, BUT NO RUNNING PROCESS")
    }
}
