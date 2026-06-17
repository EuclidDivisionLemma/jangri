use crate::{ARCH, Syscall};
use alloc::string::String;
use hal::interrupts::InterruptHandling;

pub fn write(text: &str) {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::Write as usize;
    args.1 = text.as_ptr().addr();
    args.2 = text.len();
    assert!(ARCH::make_sycall(args).unwrap() == args.2);
}

fn read_char() -> char {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::ReadChar as usize;
    ARCH::make_sycall(args).unwrap() as u8 as char
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
