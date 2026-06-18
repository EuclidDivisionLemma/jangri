#![no_std]

#[cfg(target_arch = "riscv64")]
#[cfg(feature = "user")]
use core::arch::global_asm;
#[cfg(feature = "user")]
use core::panic::PanicInfo;
use core::ptr::{write_bytes, write_volatile};

use hal::{
    constants::{ERROR_PAGE, PAGE_SIZE, TRAPFRAME},
    error::{Error, Result},
    interrupts::InterruptHandling,
};
#[cfg(feature = "user")]
use talc::{DefaultBinning, base::binning::Binning, source::Source, sync::TalcLock};

use crate::memory::{UserMemorySlice, want_memory};

extern crate alloc;

pub mod io;

pub mod memory;

#[cfg(not(feature = "user"))]
pub mod ramfs;

pub type ARCH = riscv_arch::Riscv;

pub enum Syscall {
    WantMemory = 1,
    Write = 2,
    ReadChar = 3,
    Exit = 4,
    Spawn = 5,
}

pub const SYSCALLS: [(Syscall, usize); 5] = [
    (Syscall::WantMemory, 1),
    (Syscall::Write, 2),
    (Syscall::ReadChar, 3),
    (Syscall::Exit, 4),
    (Syscall::Spawn, 5),
];

impl TryFrom<usize> for Syscall {
    type Error = hal::error::Error;

    fn try_from(value: usize) -> Result<Self> {
        Ok(match value {
            1 => Syscall::WantMemory,
            2 => Syscall::Write,
            3 => Syscall::ReadChar,
            4 => Syscall::Exit,
            5 => Syscall::Spawn,
            _ => return Err(hal::error::Error::InvalidSyscallNo(value)),
        })
    }
}

pub fn exit(status: usize) -> ! {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::Exit as usize;
    args.1 = status;
    ARCH::make_sycall(args).unwrap();
    unreachable!()
}

pub fn spawn(executable: usize, size: usize) -> Result<()> {
    let mut args = hal::interrupts::SyscallArgs::default();
    args.0 = Syscall::Spawn as usize;
    args.1 = executable;
    args.2 = size;
    ARCH::make_sycall(args)
        .map(|_| ())
        .map_err(|_| unsafe { get_error() })
}

pub unsafe fn get_error() -> Error {
    let e = unsafe { *(ERROR_PAGE as *const Error) };
    unsafe {
        write_bytes(ERROR_PAGE as *mut u8, 0, PAGE_SIZE);
    }
    e
}

#[cfg(feature = "user")]
#[derive(Debug)]
pub struct CustomSource;

#[cfg(feature = "user")]
impl CustomSource {
    const fn empty() -> Self {
        Self
    }
}

#[cfg(feature = "user")]
unsafe impl Source for CustomSource {
    fn acquire<B: Binning>(
        talc: &mut talc::base::Talc<Self, B>,
        layout: core::alloc::Layout,
    ) -> core::result::Result<(), ()> {
        let (start, size) = want_memory(layout.size()).map_err(|_| ())?;
        unsafe {
            talc.claim(start as *mut u8, size).unwrap();
        }
        Ok(())
    }
}

#[cfg(feature = "user")]
#[global_allocator]
pub static ALLOCATOR: TalcLock<spin::mutex::Mutex<()>, CustomSource, DefaultBinning> =
    TalcLock::new(CustomSource::empty());

#[cfg(feature = "user")]
#[cfg(target_arch = "riscv64")]
global_asm!(
    r"
    .global _start
    _start:
        call main
        li a7, 4
        li a0, 0
        ecall
    loop:
        j loop
    "
);

#[cfg(feature = "user")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::write("PANIC");
    exit(1);
}
