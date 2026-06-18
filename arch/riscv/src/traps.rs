use core::{
    arch::{asm, global_asm},
    fmt::Debug,
    mem::{self, transmute},
    ptr::write_bytes,
};
use hal::{
    constants::{ERROR_PAGE, PAGE_SIZE, TIME_SLICE, TRAMPOLINE, TRAPFRAME},
    error::Error,
    interrupts::{InterruptHandling, SyscallArgs},
};
use riscv::{
    interrupt::{
        Trap,
        supervisor::{Exception, Interrupt},
    },
    register::stvec::Stvec,
};

use crate::{
    Riscv,
    plic::{self, UART_ID},
    uart,
};

unsafe extern "C" {
    pub fn handle_traps_from_supervisor_mode();
}

global_asm!(
    r#"
    .section .text.handle_traps_from_supervisor_mode
    .global handle_traps_from_supervisor_mode
    .global supervisor_trap
    .align 4


    handle_traps_from_supervisor_mode:
        addi sp, sp, -256

        # save caller-saved registers
        sd ra, 0(sp)
        sd gp, 16(sp)
        sd tp, 24(sp)
        sd t0, 32(sp)
        sd t1, 40(sp)
        sd t2, 48(sp)
        sd a0, 56(sp)
        sd a1, 64(sp)
        sd a2, 72(sp)
        sd a3, 80(sp)
        sd a4, 88(sp)
        sd a5, 96(sp)
        sd a6, 104(sp)
        sd a7, 112(sp)
        sd t3, 120(sp)
        sd t4, 128(sp)
        sd t5, 136(sp)
        sd t6, 144(sp)

        call supervisor_trap

        # restore caller-saved registers
        ld ra, 0(sp)
        ld gp, 16(sp)
        ld tp, 24(sp)
        ld t0, 32(sp)
        ld t1, 40(sp)
        ld t2, 48(sp)
        ld a0, 56(sp)
        ld a1, 64(sp)
        ld a2, 72(sp)
        ld a3, 80(sp)
        ld a4, 88(sp)
        ld a5, 96(sp)
        ld a6, 104(sp)
        ld a7, 112(sp)
        ld t3, 120(sp)
        ld t4, 128(sp)
        ld t5, 136(sp)
        ld t6, 144(sp)

        addi sp, sp, 256
        sret
    "#
);

#[repr(C)]
#[repr(align(16))]
#[derive(Default, Debug, Clone)]
pub struct TrapFrame {
    pub ra: usize, // 0
    pub sp: usize, // 8

    gp: usize, // 16
    tp: usize, // 24
    t0: usize, // 32
    t1: usize, // 40
    t2: usize, // 48

    pub s0: usize, // 56
    s1: usize,     // 64
    pub a0: usize, // 72
    pub a1: usize, // 80
    pub a2: usize, // 88
    a3: usize,     // 96
    a4: usize,     // 104
    a5: usize,     // 112
    a6: usize,     // 120
    pub a7: usize, // 128

    s2: usize,  // 136
    s3: usize,  // 144
    s4: usize,  // 152
    s5: usize,  // 160
    s6: usize,  // 168
    s7: usize,  // 176
    s8: usize,  // 184
    s9: usize,  // 192
    s10: usize, // 200
    s11: usize, // 208

    t3: usize, // 216
    t4: usize, // 224
    t5: usize, // 232
    t6: usize, // 240

    pub sepc: usize,       // 248
    pub page_table: usize, // 256
    /// CAUTION: Holds the low address of the stack
    pub kernel_stack: usize, // 264
    pub kernel_page_table: usize, // 272
    pub user_trap_address: usize, // 280
    pub satp: usize,       // 288
}

impl hal::interrupts::TrapFrame for TrapFrame {
    fn set_success_indicator(this: *mut Self, return_value: usize) {
        unsafe {
            (*this).a0 = return_value;
            write_bytes(ERROR_PAGE as *mut u8, 0, PAGE_SIZE);
        }
    }

    fn set_return_value(this: *mut Self, value: usize) {
        unsafe {
            (*this).a1 = value;
        }
    }

    fn set_return_address(this: *mut Self, addr: usize) {
        unsafe {
            (*this).ra = addr;
        }
    }

    fn set_sp(this: *mut Self, addr: usize) {
        unsafe {
            (*this).sp = addr;
        }
    }

    fn set_entry_point(this: *mut Self, addr: usize) {
        unsafe {
            (*this).sepc = addr;
        }
    }
}

impl InterruptHandling for Riscv {
    type TRAPFRAME = TrapFrame;

    unsafe fn enable_interrupts() {
        unsafe {
            riscv::interrupt::supervisor::enable();
        }
    }

    fn disable_interrupts() {
        riscv::interrupt::supervisor::disable();
    }

    fn set_next_timer_interrupt(time: usize) {
        unsafe { asm!("csrw stimecmp, {}", in(reg) riscv::register::time::read() + time) }
    }

    fn are_interrupts_enabled() -> bool {
        riscv::register::sstatus::read().sie()
    }

    fn initialise_traps() {
        unsafe {
            riscv::register::stvec::write(Stvec::new(
                handle_traps_from_supervisor_mode as unsafe extern "C" fn() as usize,
                riscv::register::stvec::TrapMode::Direct,
            ));
            riscv::register::sscratch::write(TRAPFRAME);
            riscv::interrupt::supervisor::enable();
            riscv::register::sie::set_stimer();
            riscv::register::sie::set_sext();
        }

        Self::set_next_timer_interrupt(TIME_SLICE);
    }

    fn wfi() {
        riscv::asm::wfi();
    }

    fn is_timer_interrupt() -> bool {
        let cause = riscv::register::scause::read();

        if cause.cause() == Trap::Interrupt(Interrupt::SupervisorTimer as usize) {
            true
        } else {
            false
        }
    }

    fn is_external_interrupt() -> bool {
        let cause = riscv::register::scause::read();

        if cause.cause() == Trap::Interrupt(Interrupt::SupervisorExternal as usize) {
            true
        } else {
            false
        }
    }

    fn is_software_interrupt() -> bool {
        let cause = riscv::register::scause::read();

        if cause.cause() == Trap::Interrupt(Interrupt::SupervisorSoft as usize) {
            true
        } else {
            false
        }
    }

    fn is_exception() -> bool {
        let cause = riscv::register::scause::read();

        if let Trap::Exception(_) = cause.cause() {
            if Self::is_syscall() { false } else { true }
        } else {
            false
        }
    }

    fn handle_external_interrupt() {
        let id = plic::claim();

        if id == UART_ID {
            uart::handle_interrupt();
        }

        if id != 0 {
            plic::complete(id);
        }
    }

    fn is_syscall() -> bool {
        let cause = riscv::register::scause::read();

        if cause.cause() == Trap::Exception(Exception::UserEnvCall as usize) {
            true
        } else {
            false
        }
    }

    fn handle_syscall(trapframe: *mut Self::TRAPFRAME) -> hal::interrupts::SyscallArgs {
        let syscall_no = unsafe { (*trapframe).a7 };
        let mut args = SyscallArgs::default();
        args.0 = syscall_no;

        unsafe {
            (*trapframe).sepc += 4;

            args.1 = (*trapframe).a0;
            args.2 = (*trapframe).a1;
            args.3 = (*trapframe).a2;
        }

        args
    }

    fn cause() -> impl Debug {
        let cause = riscv::register::scause::read();
        cause.cause()
    }

    fn set_user_mode_trap_handler() {
        unsafe {
            riscv::register::stvec::write(riscv::register::stvec::Stvec::new(
                transmute::<usize, fn() -> !>(TRAMPOLINE) as usize,
                riscv::register::stvec::TrapMode::Direct,
            ));
        }
    }

    fn set_supervisor_mode_trap_handler() {
        unsafe {
            riscv::register::stvec::write(riscv::register::stvec::Stvec::new(
                handle_traps_from_supervisor_mode as unsafe extern "C" fn() as usize,
                riscv::register::stvec::TrapMode::Direct,
            ));
        }
    }

    fn set_up_supervisor_to_user_mode_transition(trapframe: *const Self::TRAPFRAME) {
        unsafe {
            riscv::register::sepc::write((*trapframe).sepc);
            riscv::register::sstatus::set_spp(riscv::register::sstatus::SPP::User);
            riscv::register::sstatus::set_spie();
        }
    }

    fn make_sycall(args: SyscallArgs) -> Result<usize, ()> {
        unsafe {
            asm!(
                "mv a7, {}",
                "mv a0, {}",
                "mv a1, {}",
                "mv a2, {}",
                "ecall",
                in(reg) args.0,
                in(reg) args.1,
                in(reg) args.2,
                in(reg) args.3
            );

            let a0: usize;
            asm!("mv {}, a0", out(reg) a0);
            let a1: usize;
            asm!("mv {}, a1", out(reg) a1);

            if a0 == 0 {
                Ok(a1)
            } else if a0 == 1 {
                Err(())
            } else {
                panic!("Unexpected return value after syscall")
            }
        }
    }

    fn intpc() -> impl Debug {
        riscv::register::sepc::read()
    }

    fn intmem() -> impl Debug {
        riscv::register::stval::read()
    }
}
