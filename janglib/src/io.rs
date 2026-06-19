use crate::{KUCOM_PAGE, Syscall, SyscallResult, make_syscall, write_syscall};
use alloc::string::String;
use core::{arch::asm, panic::PanicInfo};
use hal::{error::Result, interrupts::InterruptHandling};

pub fn write(text: &str) {
    write_syscall(Syscall::Write(text.as_ptr().addr(), text.len()));
    make_syscall!(Syscall::Write);
    assert!(check().unwrap() == text.len());
}

fn read_char() -> char {
    write_syscall(Syscall::ReadChar);
    make_syscall!(Syscall::ReadChar);
    check().unwrap()
}

pub fn read() -> String {
    let mut s = String::new();

    #[allow(unused_assignments)]
    let mut ch = 0 as char;

    while {
        ch = read_char();
        ch != '\n'
    } {
        s.push(ch);
    }

    s
}
