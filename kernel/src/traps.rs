use core::arch::{asm, global_asm};

use alloc::format;
use riscv::register::stvec::Stvec;

use crate::{
    constants::{TIMER_EXTENION_ID, TRAP_STACK},
    syscall::stdout,
};

unsafe extern "C" {
    fn handle_traps_from_supervisor_mode();
}

pub fn set_sbi_timer(time: usize) {
    unsafe {
        asm!("rdtime a0", "add a0, a0, {}", "mv a7, {}", "li a6, 0", "ecall", in(reg) time, in(reg) TIMER_EXTENION_ID );
    }
    stdout("CALLED\n");
}

global_asm!(
    r#"
    .section .text.trap_handlers
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
#[derive(Clone, Copy)]
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
    a0: usize,
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
}

pub fn initialise_traps() {
    unsafe {
        riscv::register::stvec::write(Stvec::new(
            handle_traps_from_supervisor_mode as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
        riscv::register::sscratch::write(TRAP_STACK);
        riscv::interrupt::enable();
        set_sbi_timer(10000000);
    }
}

#[unsafe(no_mangle)]
pub fn supervisor_trap() {
    set_sbi_timer(10000000);
}
