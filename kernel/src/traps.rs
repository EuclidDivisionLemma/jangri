use core::{fmt::Debug, mem::transmute};
use hal::{constants::TRAMPOLINE, interrupts::InterruptHandling};

use alloc::{boxed::Box, format, sync::Arc};
use hal::error::Result;

use janglib::{print, println};
use riscv_arch::uart;
use spin::Once;

use crate::{
    ARCH, Mutex, TrapFrame,
    constants::{TIME_SLICE, TRAMPOLINE_OFFSET},
    global_state::GlobalState,
    process::{Process, ProcessState, yield_cpu},
    syscall::{self},
};

static GLOBAL_STATE: Once<GlobalState> = Once::new();

pub fn get_global_state() -> &'static GlobalState {
    GLOBAL_STATE.get().unwrap()
}

pub fn initialise_global_state_for_trap_handlers(state: GlobalState) -> &'static GlobalState {
    GLOBAL_STATE.call_once(|| state)
}

pub fn initialise() {
    ARCH::initialise_traps();
    ARCH::set_next_timer_interrupt(TIME_SLICE);
}

#[unsafe(no_mangle)]
pub fn supervisor_trap() {
    if ARCH::is_timer_interrupt() {
        ARCH::set_next_timer_interrupt(TIME_SLICE);
    } else if ARCH::is_external_interrupt() {
        ARCH::handle_external_interrupt();
    } else if ARCH::is_exception() {
        panic!("UNHANDLED SUPERVISOR TRAP EXCEPTION: {:?}", ARCH::cause());
    }
}

pub fn user_trap() {
    ARCH::set_supervisor_mode_trap_handler();
    let state = get_global_state();
    let process: Option<Arc<Mutex<Process>>> = state.get_current_process();

    if let Some(process) = process {
        let trapframe: *mut TrapFrame = {
            let process = process.lock();
            process.trapframe
        };

        if ARCH::is_timer_interrupt() {
            ARCH::set_next_timer_interrupt(TIME_SLICE);
            yield_cpu(state);
        } else if ARCH::is_external_interrupt() {
            ARCH::handle_external_interrupt();
        } else if ARCH::is_exception() {
            let mut current_process = process.lock();

            if ARCH::is_page_fault() {
                let a = || -> Result<()> {
                    let faulting_address: usize = ARCH::intmem();
                    if current_process.currently_unmapped_start != current_process.heap_end
                        && faulting_address >= current_process.currently_unmapped_start
                        && faulting_address < current_process.heap_end
                    {
                        let block = state
                            .allocate(current_process.heap_end - current_process.heap_start)?;
                        state.map(
                            current_process.page_table,
                            current_process.heap_start,
                            block,
                            current_process.heap_end - current_process.heap_start,
                            true,
                            true,
                            false,
                            true,
                        )?;
                    }
                    Ok(())
                };

                if let Err(e) = a() {
                    println!(
                        "Error Occured: Terminating process name = {}, pid = {}, error = {:?}",
                        current_process.name, current_process.id, e
                    );
                    syscall::exit(state, Err(Box::new(e)));
                } else {
                    current_process.currently_unmapped_start = current_process.heap_end;
                }
            } else {
                let cause = Box::new(ARCH::cause());
                let name = current_process.name.clone();
                let id = current_process.id;
                println!(
                    "Exception Occured: Terminating process name = {}, pid = {}, cause = {:?}, \
                Faulting instruction address = {:?}, Faulting memory address = {:?}",
                    name,
                    id,
                    &cause,
                    ARCH::intpc(),
                    ARCH::intmem(),
                );
                syscall::exit(state, Err(cause));
            }
        } else if ARCH::is_syscall() {
            syscall::handle(state);
        }

        set_up_supervisor_to_user_mode_transition(state)
            .expect("TRAP ERROR - CONTEXT NONE WHILE RETURNING TO USER MODE");

        unsafe {
            let return_to_user_mode_ptr: fn(usize) -> ! = transmute(TRAMPOLINE + TRAMPOLINE_OFFSET);
            return_to_user_mode_ptr(trapframe.addr());
        }
    } else {
        panic!("USER TRAP, BUT NO RUNNING PROCESS")
    }
}

pub fn set_up_supervisor_to_user_mode_transition(state: &GlobalState) -> Result<()> {
    // Disable interrupts because we are changing stvec to point to
    // `handle_traps_from_user_mode` and we don't want an interrupt
    // to be handled by it while we are still in supervisor mode
    state.disable_interrupts();

    ARCH::set_user_mode_trap_handler();

    let process = state.get_current_process().unwrap();
    let process = process.lock();
    let trapframe = process.trapframe;
    ARCH::set_up_supervisor_to_user_mode_transition(trapframe);

    Ok(())
}
