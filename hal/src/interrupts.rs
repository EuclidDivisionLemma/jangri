use core::fmt::Debug;

use crate::error;

pub trait InterruptHandling {
    type TRAPFRAME: TrapFrame;

    unsafe fn enable_interrupts();
    fn disable_interrupts();
    fn set_next_timer_interrupt(time: usize);
    fn are_interrupts_enabled() -> bool;
    fn initialise_traps();
    fn wfi();
    fn is_timer_interrupt() -> bool;
    fn is_external_interrupt() -> bool;
    fn is_software_interrupt() -> bool;
    fn is_exception() -> bool;
    fn is_syscall() -> bool;
    fn handle_external_interrupt();
    fn handle_syscall(trapframe: *mut Self::TRAPFRAME) -> SyscallArgs;
    fn cause() -> impl Debug;
    fn set_user_mode_trap_handler();
    fn set_supervisor_mode_trap_handler();
    fn set_up_supervisor_to_user_mode_transition(trapframe: *const Self::TRAPFRAME);
    fn make_sycall(args: SyscallArgs) -> Result<usize, usize>;
}

pub trait TrapFrame {
    fn set_success_indicator(this: *mut Self, status: usize);
    fn set_error(this: *mut Self, error: error::Error);
    fn set_return_value(this: *mut Self, value: usize);
}

#[derive(Default, Clone, Copy)]
pub struct SyscallArgs(pub usize, pub usize, pub usize, pub usize);
