use crate::{KUCOM_PAGE, Syscall, SyscallResult, get_result, make_syscall, println, write_syscall};
use alloc::{format, string::String};
use core::{arch::asm, panic::PanicInfo};
use hal::{error::Result, interrupts::InterruptHandling};

pub fn write(text: &str) {
    if text.is_empty() {
        return;
    }
    write_syscall(Syscall::Write(text.as_ptr().addr(), text.len()));
    make_syscall!(Syscall::Write);
    assert!(check().unwrap() == text.len());
}

fn read_char() -> Option<char> {
    write_syscall(Syscall::ReadChar);
    make_syscall!(Syscall::ReadChar);
    check().unwrap()
}

pub fn read() -> String {
    let mut s = String::new();
    loop {
        if let Some(ch) = read_char() {
            if ch == '\n' {
                break;
            } else if ch == 0x08 as char {
                let _ = s.pop().unwrap();
            } else {
                s.push(ch);
            }
        }
    }

    s
}
