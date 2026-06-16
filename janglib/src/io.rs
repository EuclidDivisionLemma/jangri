use crate::Syscall;
use alloc::string::String;

pub fn write(text: &str) {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::Write as usize;
    args.1 = text.as_ptr().addr();
    args.2 = text.len();
    assert!(hal::interrupts::InterruptHandling::make_sycall(args).unwrap() == args.2);
}

fn read_char() -> char {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::ReadChar as usize;
    hal::interrupts::InterruptHandling::make_sycall(args).unwrap() as u8 as char
}

pub fn read() -> String {
    let mut s = String::new();
    let mut ch = 0 as char;

    while {
        ch = read_char();
        ch != '\n'
    } {
        s.push(ch);
    }

    s
}
