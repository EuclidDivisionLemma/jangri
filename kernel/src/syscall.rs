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
    error::Error,
    global_state::GlobalState,
    pipe::allocate_pipe,
    process::{self, ProcessState},
    scheduler::switch_to_scheduler_context,
    vm::{self, translate_virtual_address},
};

pub const SYSCALLS: [(Syscall, fn(&GlobalState, SyscallArgs) -> usize); 6] = [
    (Syscall::Read, read),
    (Syscall::Write, write),
    (Syscall::Sbrk, sbrk),
    (Syscall::Pipe, pipe),
    (Syscall::Exit, exit),
    (Syscall::Close, close),
];

const ENXIO: isize = 6;
const EBADF: isize = 9;
const EPIPE: isize = 32;

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
    let fd = args.1;
    let mut num_bytes = args.3;
    let current_process = state.get_current_process().unwrap();
    let current_process = current_process.lock();

    let ptr = translate_virtual_address(state, current_process.page_table, args.2).unwrap();

    if fd == 1 || fd == 2 {
        let text = unsafe { CStr::from_ptr(ptr as *const u8).to_str().unwrap() };
        num_bytes = text.len();
        uart::console_write(text);
    } else {
        let pipes = state.pipes.read();
        let pipe = match pipes.get(&fd) {
            Some(pipe) => pipe,
            None => return -ENXIO as usize,
        };

        let mut pipe = pipe.lock();

        if fd == pipe.reader {
            return -EBADF as usize;
        }

        let buf = unsafe { slice::from_raw_parts(ptr as *const u8, num_bytes) };
        drop(current_process);

        if let Err(e) = pipe.write(state, buf) {
            match e.downcast_ref().unwrap() {
                Error::PipeWriterClosed | Error::PipeReaderClosed => return -EPIPE as usize,
                _ => panic!(),
            }
        }
    }
    num_bytes
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
    let fd = args.1;

    let buf = translate_virtual_address(state, current_process.lock().page_table, args.2).unwrap()
        as *mut u8;

    let mut read = 0;

    if fd == 0 {
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
    } else {
        let pipes = state.pipes.read();

        if let Some(pipe) = pipes.get(&fd) {
            let mut pipe = pipe.lock();

            if fd != pipe.reader {
                return -ENXIO as usize;
            }

            let bytes = pipe.read(state, num_bytes);
            unsafe {
                buf.copy_from(bytes.as_ptr(), num_bytes);
            }

            num_bytes
        } else {
            -ENXIO as usize
        }
    }
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

fn pipe(state: &GlobalState, args: SyscallArgs) -> usize {
    let current_process = state.get_current_process().unwrap();
    let current_process = current_process.lock();

    let reader_address =
        translate_virtual_address(state, current_process.page_table, args.1).unwrap() as *mut c_int;

    let writer_address = translate_virtual_address(
        state,
        current_process.page_table,
        args.1 + size_of::<c_int>(),
    )
    .unwrap() as *mut c_int;

    let pipe = allocate_pipe(state);
    let pipe = pipe.lock();

    unsafe {
        write_volatile(reader_address, pipe.reader as i32);
        write_volatile(writer_address, pipe.writer as i32);
    }

    0
}

pub fn close(state: &GlobalState, args: SyscallArgs) -> usize {
    let fd = args.1;

    if fd == 0 || fd == 1 || fd == 2 {
        return 0;
    };

    let mut pipes = state.pipes.write();

    {
        let pipe = if let Some(pipe) = pipes.get(&fd) {
            pipe
        } else {
            return -EBADF as usize;
        };

        let mut pipe = pipe.lock();

        if fd == pipe.reader {
            pipe.read_end_open = false;
            process::wake_up(state, (&raw const pipe.write_offset).addr());
        } else if fd == pipe.writer {
            pipe.write_end_open = true;
            process::wake_up(state, (&raw const pipe.read_offset).addr());
        }
    }
    pipes.remove(&fd).unwrap();

    0
}
