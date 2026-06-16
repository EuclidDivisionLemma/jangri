#![no_std]

extern crate alloc;

pub mod io;
pub mod memory;

pub enum Syscall {
    WantMemory = 0,
    Write = 1,
    ReadChar = 2,
    Exit = 3,
}

pub const SYSCALLS: [(Syscall, usize); 4] = [
    (Syscall::WantMemory, 0),
    (Syscall::Write, 1),
    (Syscall::ReadChar, 2),
    (Syscall::Exit, 3),
];

impl TryFrom<usize> for Syscall {
    type Error = hal::error::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Syscall::WantMemory,
            1 => Syscall::Write,
            2 => Syscall::ReadChar,
            3 => Syscall::Exit,
            _ => return Err(hal::error::Error::InvalidSyscallNo(value)),
        })
    }
}
