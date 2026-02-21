use core::fmt::Debug;

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
}

pub trait TrapFrame {
    fn set_return_value_after_syscall(this: *mut Self, return_value: usize);
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Syscall {
    Read = 100,
    Write = 200,
    Sbrk = 300,
    Pipe = 400,
    Exit = 500,
    Close = 600,
}

#[derive(Default)]
pub struct SyscallArgs(pub usize, pub usize, pub usize, pub usize);
