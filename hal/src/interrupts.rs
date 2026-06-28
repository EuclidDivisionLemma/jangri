use core::fmt::Debug;

use crate::error::{self, Error};

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
    fn cause() -> impl Debug;
    fn intpc() -> impl Debug;
    fn intmem() -> usize;
    fn set_user_mode_trap_handler();
    fn set_supervisor_mode_trap_handler();
    fn set_up_supervisor_to_user_mode_transition(trapframe: *const Self::TRAPFRAME);
    fn handle_syscall(trapframe: *mut Self::TRAPFRAME);
    fn is_page_fault() -> bool;
}

pub trait TrapFrame {
    fn set_return_address(this: *mut Self, addr: usize);
    fn set_sp(this: *mut Self, addr: usize);
    fn set_entry_point(this: *mut Self, addr: usize);
}

pub fn make_syscall() {
    #[cfg(target_arch = "riscv64")]
    use core::arch::asm;
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("ecall");
    }
}
