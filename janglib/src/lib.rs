#![no_std]

extern crate alloc;

pub mod error;
pub mod io;
pub mod memory;

pub enum Syscall {
    WantMemory = 0,
    Write = 1,
    ToPhysicalAddress = 3,
    ReadChar = 4,
}

pub const SYSCALLS: [(Syscall, usize); 4] = [
    (Syscall::WantMemory, 0),
    (Syscall::Write, 1),
    (Syscall::ToPhysicalAddress, 3),
    (Syscall::ReadChar, 4),
];
