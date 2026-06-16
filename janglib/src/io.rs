use crate::Syscall;
use alloc::string::String;
use hal::interrupts::InterruptHandling;

pub fn write<I: InterruptHandling>(text: &str) {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::Write as usize;
    args.1 = text.as_ptr().addr();
    args.2 = text.len();
    assert!(I::make_sycall(args).unwrap() == args.2);
}

fn read_char<I: InterruptHandling>() -> char {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::ReadChar as usize;
    I::make_sycall(args).unwrap() as u8 as char
}

pub fn read<I: InterruptHandling>() -> String {
    let mut s = String::new();

    #[allow(unused_assignments)]
    let mut ch = 0 as char;

    while {
        ch = read_char::<I>();
        ch != '\n'
    } {
        s.push(ch);
    }

    s
}
