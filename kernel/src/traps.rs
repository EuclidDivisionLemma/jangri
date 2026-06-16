use core::mem::transmute;
use hal::interrupts::InterruptHandling;

use alloc::{boxed::Box, format};
use hal::error::Result;

use riscv_arch::uart;
use spin::Once;

use crate::{
    ARCH,
    constants::{TIME_SLICE, TRAMPOLINE, TRAMPOLINE_OFFSET},
    global_state::GlobalState,
    process::{wake_up, yield_cpu},
    syscall::{self, stdout},
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
    let process = state.get_current_process();

    if let Some(locked_process) = process {
        let trapframe;

        let process = locked_process.lock();
        trapframe = process.trapframe;
        drop(process);

        if ARCH::is_timer_interrupt() {
            ARCH::set_next_timer_interrupt(TIME_SLICE);
            yield_cpu(state);
        } else if ARCH::is_external_interrupt() {
            ARCH::handle_external_interrupt();
        } else if ARCH::is_exception() {
            let cause = Box::new(ARCH::cause());
            let process = state.get_current_process().unwrap();
            let current_process = process.lock();
            let name = current_process.name.clone();
            let id = current_process.id;
            drop(current_process);
            uart::console_write(&format!(
                "Exception Occured: Terminating process name = {}, pid = {}, cause = {:?}\n",
                name, id, &cause,
            ));
            wake_up(state, id);
            let mut current_process = process.lock();
            current_process.process_state = crate::process::ProcessState::Terminated {
                return_value: Err(cause),
            };
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
