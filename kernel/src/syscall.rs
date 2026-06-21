use core::{arch::asm, ffi::c_int, fmt::Debug, mem, ptr::write_volatile, slice};

use alloc::{boxed::Box, format, string::ToString, sync::Arc, vec::Vec};
use hal::{
    constants::{KUCOM_PAGE, PAGE_SIZE, STACK_GUARD},
    error::{Error, Result},
    interrupts::InterruptHandling,
};
use janglib::{Syscall, SyscallInfo, SyscallResult, memory::UserMemorySlice};
use ringbuffer::RingBuffer;
use riscv_arch::uart::{self, INPUT_BUFFER};

use crate::{
    ARCH, Mutex, TrapFrame,
    global_state::GlobalState,
    print,
    process::{
        self, Process, ProcessState, assign_process, prepare_first_time_execution, yield_cpu,
    },
    scheduler::switch_to_scheduler_context,
};

use hal::vm::align_to_page_size;

pub fn handle(state: &'static GlobalState) {
    let locked_process: Arc<Mutex<Process>> = state
        .get_current_process()
        .expect("Syscalld but no running process");
    let (trapframe, page_table): (*mut TrapFrame, usize) = {
        let process = locked_process.lock();
        (process.trapframe, process.page_table)
    };
    ARCH::handle_syscall(trapframe);

    let syscall = {
        let mut syscall_page = UserMemorySlice::<false, _>::new(
            KUCOM_PAGE,
            page_table,
            PAGE_SIZE,
            |va: usize, page_table: usize| state.va2pa(page_table, va),
        );
        let syscall_vec = syscall_page.read_to_vec();
        let mut syscall_buf = [0u8; size_of::<SyscallInfo>()];
        syscall_buf.copy_from_slice(&syscall_vec[0..size_of::<SyscallInfo>()]);
        match unsafe { mem::transmute::<_, SyscallInfo>(syscall_buf) } {
            SyscallInfo::Syscall(syscall) => syscall,
            SyscallInfo::SyscallResult(syscall_result) => {
                panic!(
                    "Expected Syscall, found SyscallInfo::SyscallResult::{:?}\n",
                    syscall_result
                )
            }
            SyscallInfo::Empty => panic!("Expected Syscall, found SyscallInfo::Empty\n"),
        }
    };

    let result = || -> SyscallInfo {
        match syscall {
            Syscall::WantMemory(size) => {
                SyscallInfo::SyscallResult(SyscallResult::WantMemory(want_memory(state, size)))
            }
            Syscall::Write(start, len) => {
                let slice = janglib::memory::UserMemorySlice::<false, _>::new(
                    start,
                    page_table,
                    len,
                    |va, page_table| state.va2pa(page_table, va),
                );
                print!(str::from_utf8(slice.read()).unwrap());
                SyscallInfo::SyscallResult(SyscallResult::Write(Ok(len)))
            }
            Syscall::ReadChar => {
                let mut ibf = INPUT_BUFFER.lock();

                SyscallInfo::SyscallResult(SyscallResult::ReadChar(Ok(
                    if let Some(ch) = ibf.dequeue() {
                        Some(ch as char)
                    } else {
                        None
                    },
                )))
            }
            Syscall::Exit(status) => {
                let status: core::result::Result<usize, Box<dyn Debug>> = match status {
                    Ok(v) => Ok(v),
                    Err(e) => Err(Box::new(e)),
                };
                exit(state, status);
                SyscallInfo::SyscallResult(SyscallResult::Exit)
            }
            Syscall::Spawn(name_start, name_len, start, len, wait) => {
                let s = || -> Result<usize> {
                    let name = UserMemorySlice::<false, _>::new(
                        name_start,
                        page_table,
                        name_len,
                        |start, page_table| state.va2pa(page_table, start),
                    );
                    let name = str::from_utf8(name.read()).expect("Invalid UTF 8");
                    let image = UserMemorySlice::<false, _>::new(
                        start,
                        page_table,
                        len,
                        |start, page_table| state.va2pa(page_table, start),
                    )
                    .read_to_vec();
                    let child: Arc<Mutex<Process>> = assign_process(state, name, image)?;
                    {
                        let parent = Some(Arc::downgrade(&locked_process));
                        let mut process = child.lock();
                        process.parent = parent;
                    }
                    let mut current_process = locked_process.lock();
                    current_process.children.push(child.clone());

                    let child_pid = {
                        let c = child.lock();
                        c.id
                    };

                    if wait {
                        current_process.process_state = ProcessState::Waiting {
                            waiting_for: child_pid,
                        };
                    }

                    Ok(child_pid)
                };

                SyscallInfo::SyscallResult(SyscallResult::Spawn(s()))
            }
            Syscall::Yield => {
                yield_cpu(state);
                SyscallInfo::SyscallResult(SyscallResult::Yield)
            }
        }
    };
    let result = result();

    let mut result_page = UserMemorySlice::<true, _>::new(
        KUCOM_PAGE,
        page_table,
        PAGE_SIZE,
        |va: usize, page_table: usize| state.va2pa(page_table, va),
    );
    let result_buf: [u8; size_of::<SyscallInfo>()] =
        unsafe { mem::transmute::<_, [u8; size_of::<SyscallInfo>()]>(result) };
    result_page.write(result_buf.as_slice());
}

pub fn want_memory(state: &GlobalState, size: usize) -> Result<(usize, usize)> {
    let process: Arc<Mutex<Process>> = state.get_current_process().unwrap();
    let mut process = process.lock();
    let va = process.heap_end;

    assert!(size % PAGE_SIZE == 0);

    let num_pages = size / PAGE_SIZE;

    if va + num_pages * PAGE_SIZE >= STACK_GUARD {
        return Err(Error::MemoryNotAvailable);
    }
    process.heap_end += num_pages * PAGE_SIZE;
    Ok((process.heap_end - num_pages * PAGE_SIZE, size))
}

fn remove_from_vec(v: &mut Vec<Arc<Mutex<Process>>>, id: usize) {
    let v = v.iter_mut().filter_map(|e: &mut Arc<Mutex<Process>>| {
        let pid = {
            let p = e.lock();
            p.id
        };
        if pid == id { None } else { Some(e) }
    });
}

pub fn exit(state: &GlobalState, status: core::result::Result<usize, Box<dyn Debug>>) -> ! {
    let current_process: Arc<Mutex<Process>> = state.get_current_process().unwrap();
    let (parent, pid) = {
        let mut current_process = current_process.lock();
        current_process.process_state = ProcessState::Terminated {
            return_value: status,
        };
        (current_process.parent.clone(), current_process.id)
    };

    if let Some(parent) = parent
        && let Some(parent) = &parent.upgrade()
    {
        let mut parent = parent.lock();
        {
            let current_process = current_process.lock();
            remove_from_vec(&mut parent.children, current_process.id);
            parent.children.extend(current_process.children.clone());
        }
        if let ProcessState::Waiting {
            waiting_for: wait_for,
        } = &parent.process_state
            && *wait_for == pid
        {
            parent.process_state = ProcessState::Ready;
        }
    } else {
        let mut current_process = current_process.lock();
        let _ = current_process
            .children
            .iter_mut()
            .map(|e: &mut Arc<Mutex<Process>>| {
                let mut p = e.lock();
                p.parent = None
            });
    }
    {
        let mut current_process = current_process.lock();
        current_process.children.clear();
    }
    // currently clean up fails
    // state
    //     .cleanup_page_table(current_process.page_table)
    //     .unwrap();

    switch_to_scheduler_context(state);
    unreachable!("EXIT UNREACHABLE");
}
