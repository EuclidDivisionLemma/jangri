use core::{
    arch::{asm, global_asm},
    mem::transmute,
};

use riscv::{
    interrupt::{Trap, supervisor::Interrupt},
    register::stvec::Stvec,
};

use crate::{
    constants::{TIMER_EXTENION_ID, TRAMPOLINE, TRAPFRAME},
    error::{Error, Result},
    process::{CURRENT_PROCESS, yield_cpu},
    syscall::{self, stdout},
};

unsafe extern "C" {
    pub safe fn handle_traps_from_supervisor_mode();
    pub safe fn return_to_user_mode(trapframe: usize);
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

pub fn set_sbi_timer(time: usize) {
    unsafe {
        asm!("rdtime a0",
            "add a0, a0, {}",
            "mv a7, {}",
            "li a6, 0",
            "ecall",
            in(reg) time,
            in(reg) TIMER_EXTENION_ID );
    }
    stdout("CALLED\n");
}

#[repr(C)]
#[repr(align(16))]
#[derive(Clone, Copy, Default)]
pub struct TrapFrame {
    ra: usize,
    sp: usize,

    gp: usize,
    tp: usize,
    t0: usize,
    t1: usize,
    t2: usize,

    s0: usize,
    s1: usize,
    pub a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,

    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,

    t3: usize,
    t4: usize,
    t5: usize,
    t6: usize,

    pub sepc: usize,
    pub page_table: usize,
    pub kernel_stack: usize,
    pub kernel_page_table: usize,
}

pub fn initialise_traps() {
    unsafe {
        riscv::register::stvec::write(Stvec::new(
            handle_traps_from_supervisor_mode as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
        riscv::register::sscratch::write(TRAPFRAME);
    }
}

#[unsafe(no_mangle)]
pub fn supervisor_trap() {
    let cause = riscv::register::scause::read();

    if cause.is_interrupt() && cause.cause() == Trap::Interrupt(Interrupt::SupervisorTimer as usize)
    {
        todo!()
    }
}

#[unsafe(no_mangle)]
pub fn user_trap() {
    let cause = riscv::register::scause::read();
    let sepc = riscv::register::sepc::read();

    if let Some(process) = unsafe { &mut crate::process::CURRENT_PROCESS } {
        process
            .trapframe
            .as_mut()
            .expect("TRAPFRAME NONE WHILE HANDLING USER TRAP")
            .sepc = sepc;

        if cause.is_interrupt()
            && cause.cause() == Trap::Interrupt(Interrupt::SupervisorSoft as usize)
        {
            unsafe {
                riscv::interrupt::enable();
            }
            syscall::handle();
        } else if cause.is_interrupt()
            && cause.cause() == Trap::Interrupt(Interrupt::SupervisorTimer as usize)
        {
            yield_cpu();
        } else {
            panic!("UNKNOWN INTERRUPT");
        }

        set_up_supervisor_to_user_mode_transition()
            .expect("TRAP ERROR - CONTEXT NONE WHILE RETURNING TO USER MODE");
    } else {
        panic!("USER TRAP, BUT NO RUNNING PROCESS")
    }
}

pub fn set_up_supervisor_to_user_mode_transition() -> Result<()> {
    // Disable interrupts because we are changing stvec to point to
    // `handle_traps_from_user_mode` and we don't want an interrupt
    // to be handled by it while we are still in supervisor mode
    riscv::interrupt::supervisor::disable();

    unsafe {
        riscv::register::stvec::write(riscv::register::stvec::Stvec::new(
            transmute::<usize, fn() -> !>(TRAMPOLINE) as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
    }

    unsafe {
        let process = CURRENT_PROCESS.as_ref().unwrap();
        let trapframe = process.trapframe.as_ref().ok_or(Error::TrapFrameNone)?;
        riscv::register::sepc::write(trapframe.sepc);
        riscv::register::sstatus::set_spp(riscv::register::sstatus::SPP::User);
        riscv::register::sstatus::set_spie();
    }

    Ok(())
}
