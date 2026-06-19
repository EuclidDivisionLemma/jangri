#![no_std]

use core::arch::asm;
#[cfg(target_arch = "riscv64")]
#[cfg(feature = "user")]
use core::arch::global_asm;
#[cfg(feature = "user")]
use core::panic::PanicInfo;
use core::{
    mem,
    ptr::{write_bytes, write_volatile},
};

use hal::{
    constants::{KUCOM_PAGE, PAGE_SIZE, TRAPFRAME},
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

#[derive(Debug, Clone, Copy)]
pub enum Syscall {
    WantMemory(usize),
    Write(usize, usize),
    ReadChar,
    Exit(Result<usize>),
    Spawn(usize, usize),
    Yield,
}

#[derive(Debug, Clone, Copy)]
pub enum SyscallResult {
    WantMemory(Result<(usize, usize)>),
    Write(Result<usize>),
    ReadChar(Result<char>),
    Exit,
    Spawn(Result<()>),
    Yield,
}

#[derive(Debug, Clone, Copy)]
pub enum SyscallInfo {
    Syscall(Syscall),
    SyscallResult(SyscallResult),
    Empty,
}

pub(crate) fn write_syscall(syscall: Syscall) {
    unsafe {
        *(KUCOM_PAGE as *mut SyscallInfo) = SyscallInfo::Syscall(syscall);
    }
}

pub(crate) fn get_result() -> SyscallResult {
    let result = {
        let result = unsafe { *(KUCOM_PAGE as *const SyscallInfo) };
        if let SyscallInfo::SyscallResult(r) = result {
            r
        } else {
            panic!();
        }
    };
    unsafe {
        write_volatile(KUCOM_PAGE as *mut SyscallInfo, SyscallInfo::Empty);
    }
    result
}

#[macro_export]
macro_rules! make_syscall {
    (Syscall::WantMemory) => {
        hal::interrupts::make_syscall();

        pub fn check() -> Result<(usize, usize)> {
            let result = get_result();

            match result {
                SyscallResult::WantMemory(v) => v,
                _ => panic!(),
            }
        }
    };

    (Syscall::Write) => {
        hal::interrupts::make_syscall();

        pub fn check() -> Result<usize> {
            let result = get_result();
            match result {
                SyscallResult::Write(v) => v,
                _ => panic!(),
            }
        }
    };

    (Syscall::ReadChar) => {
        hal::interrupts::make_syscall();

        pub fn check() -> Result<char> {
            let result = get_result();
            match result {
                SyscallResult::ReadChar(v) => v,
                _ => panic!(),
            }
        }
    };

    (Syscall::Exit) => {
        hal::interrupts::make_syscall();

        pub fn check() -> () {
            let result = get_result();
            match result {
                SyscallResult::Exit => (),
                _ => panic!(),
            }
        }
    };

    (Syscall::Spawn) => {
        hal::interrupts::make_syscall();

        pub fn check() -> Result<()> {
            let result = get_result();
            match result {
                SyscallResult::Spawn(v) => v,
                _ => panic!(),
            }
        }
    };

    (Syscall::Yield) => {
        hal::interrupts::make_syscall();
    };
}

pub fn exit(status: Result<usize>) -> ! {
    write_syscall(Syscall::Exit(status));
    make_syscall!(Syscall::Exit);
    check();
    unreachable!()
}

pub fn spawn(executable: usize, size: usize) -> Result<()> {
    write_syscall(Syscall::Spawn(executable, size));
    make_syscall!(Syscall::Spawn);
    check()
}

pub fn r#yield() {
    write_syscall(Syscall::Yield);
    make_syscall!(Syscall::Yield);
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
        call _exit
    loop:
        j loop
    "
);

#[unsafe(no_mangle)]
fn _exit() -> ! {
    exit(Ok(0))
}

#[cfg(feature = "user")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::write("PANIC");
    exit(Err(Error::ExplicitPanic));
}
