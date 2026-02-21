use core::{
    arch::asm,
    ffi::{CStr, c_int},
    ptr::{self, slice_from_raw_parts_mut, write_volatile},
    slice,
};

use alloc::format;
use hal::{
    constants::PAGE_SIZE,
    interrupts::{InterruptHandling, Syscall, SyscallArgs, TrapFrame},
};
use ringbuffer::RingBuffer;
use riscv_arch::uart::{self, INPUT_BUFFER};

use crate::{
    ARCH,
    constants::TRAMPOLINE,
    global_state::GlobalState,
    process::ProcessState,
    scheduler::switch_to_scheduler_context,
    vm::{self, translate_virtual_address},
};

pub const SYSCALLS: [(Syscall, fn(&GlobalState, SyscallArgs) -> usize); 4] = [
    (Syscall::Read, read),
    (Syscall::Write, write),
    (Syscall::Sbrk, sbrk),
    (Syscall::Exit, exit),
];

pub fn stdout<'a>(text: &'a str) {
    let chars = text.as_bytes();
    unsafe {
        for char in chars {
            asm!("li a7, 0x4442434E",
            "li a6, 2",
            "mv a0, {}",
            "ecall",
            in(reg) *char as i64);
        }
    }
}

pub fn write(state: &GlobalState, args: SyscallArgs) -> usize {
    let ptr = args.2;
    let current_process = state.get_current_process().unwrap();
    let current_process = current_process.lock();

    let text = unsafe {
        CStr::from_ptr(
            translate_virtual_address(state, current_process.page_table, ptr).unwrap() as *const u8,
        )
        .to_str()
        .unwrap()
    };
    uart::console_write(text);
    0
}

pub fn handle(state: &GlobalState) {
    if let Some(locked_process) = state.get_current_process() {
        let trapframe;

        let process = locked_process.lock();
        trapframe = process.trapframe;
        drop(process);

        let args;
        unsafe {
            args = ARCH::handle_syscall(trapframe);

            // sepc holds the program counter value at the point of trap
            // But crate::process::when the trap is due to a system call, we need to execute the next instruction

            state.enable_interrupts();
        }

        for (no, handler) in SYSCALLS {
            if args.0 == no as usize {
                let return_value = handler(state, args);
                let process = locked_process.lock();
                let trapframe = process.trapframe;

                TrapFrame::set_return_value_after_syscall(trapframe, return_value);

                return;
            }
        }
        unsafe {
            (*trapframe).a0 = -1isize as usize;
        }
    } else {
        panic!("SYSCALLd, BUT NO RUNNING PROCESS")
    }
}

fn read(state: &GlobalState, args: SyscallArgs) -> usize {
    let current_process = state.get_current_process().unwrap();
    let num_bytes = args.3;

    let buf = translate_virtual_address(state, current_process.lock().page_table, args.2).unwrap()
        as *mut u8;

    let mut read = 0;

    while read < num_bytes {
        let mut input_buffer = INPUT_BUFFER.lock();

        if let Some(byte) = input_buffer.dequeue() {
            unsafe {
                write_volatile(buf.add(read), byte);
                read += 1;

                if byte == '\n' as u8 || byte == '\r' as u8 {
                    break;
                }
            }
        }

        drop(input_buffer);
    }

    read
}

pub fn sbrk(state: &GlobalState, args: SyscallArgs) -> usize {
    const EAGAIN: isize = 11;
    let increment = args.1;
    let current_process = state.get_current_process().unwrap();
    let mut current_process = current_process.lock();

    if increment + current_process.brk < current_process.heap_end {
        let old = current_process.brk;
        current_process.brk += increment;
        return old;
    }

    assert!(current_process.brk == current_process.heap_end);

    if increment + current_process.brk < TRAMPOLINE - 11 * PAGE_SIZE {
        let num_pages = (increment + PAGE_SIZE - 1) / PAGE_SIZE;

        if let Ok(pa) = state.allocate(num_pages * PAGE_SIZE) {
            if let Ok(()) = state.map(
                current_process.page_table,
                current_process.brk,
                pa,
                num_pages * PAGE_SIZE,
                true,
                true,
                false,
                true,
            ) {
                let old = current_process.brk;
                current_process.brk += increment;
                current_process.heap_end += num_pages * PAGE_SIZE;
                return old;
            } else {
                state.deallocate(pa, num_pages * PAGE_SIZE);
            }
        }
    }

    return -EAGAIN as usize;
}

fn exit(state: &GlobalState, args: SyscallArgs) -> usize {
    let current_process = state.get_current_process().unwrap();
    let mut current_process = current_process.lock();
    current_process.process_state = ProcessState::Terminated {
        return_value: Ok(args.1 as isize),
    };
    drop(current_process);
    switch_to_scheduler_context(state);

    0
}
