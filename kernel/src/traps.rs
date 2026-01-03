use core::{
    arch::{asm, global_asm},
    cell::Cell,
    mem::transmute,
};

use riscv::{
    interrupt::{
        Trap,
        supervisor::{Exception, Interrupt},
    },
    register::{scause::Scause, stvec::Stvec},
};

use crate::{
    constants::{TIME_SLICE, TRAMPOLINE, TRAMPOLINE_OFFSET, TRAPFRAME, UART_ID},
    drivers::uart::{self, console_write},
    error::{Error, Result},
    plic,
    process::{CURRENT_PROCESS, yield_cpu},
    syscall::{self},
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

pub fn set_next_timer_interrupt(time: usize) {
    unsafe { asm!("csrw stimecmp, {}", in(reg) riscv::register::time::read() + time) }
}

#[repr(C)]
#[repr(align(16))]
#[derive(Default, Debug)]
pub struct TrapFrame {
    ra: usize,     // 0
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

    pub heap_end: Cell<usize>,
    pub brk: Cell<usize>,
}

pub fn initialise_traps() {
    unsafe {
        riscv::register::stvec::write(Stvec::new(
            handle_traps_from_supervisor_mode as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
        riscv::register::sscratch::write(TRAPFRAME);
        riscv::interrupt::supervisor::enable();
        riscv::register::sie::set_stimer();
        riscv::register::sie::set_sext();
    }

    set_next_timer_interrupt(TIME_SLICE);
}

#[unsafe(no_mangle)]
pub fn supervisor_trap() {
    let cause = riscv::register::scause::read();

    if cause.is_interrupt() && cause.cause() == Trap::Interrupt(Interrupt::SupervisorTimer as usize)
    {
        set_next_timer_interrupt(TIME_SLICE);
    } else if cause.is_interrupt()
        && cause.cause() == Trap::Interrupt(Interrupt::SupervisorExternal as usize)
    {
        let id = plic::claim();

        if id == UART_ID {
            uart::handle_interrupt();
        }

        if id != 0 {
            plic::complete(id);
        }
    } else if cause.is_exception() {
        panic!("UNHANDLED SUPERVISOR TRAP EXCEPTION: {:?}", cause);
    }
}

pub fn user_trap() {
    unsafe {
        riscv::register::stvec::write(riscv::register::stvec::Stvec::new(
            handle_traps_from_supervisor_mode as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
    }

    let cause = riscv::register::scause::read();
    let sepc = riscv::register::sepc::read();

    if let Some(process) = unsafe { &mut crate::process::CURRENT_PROCESS } {
        let trapframe = process
            .trapframe
            .as_mut()
            .expect("TRAPFRAME NONE WHILE HANDLING USER TRAP");

        trapframe.sepc = sepc;

        if cause.is_interrupt() {
            handle_interrupts(cause);
        } else if cause.is_exception() {
            handle_exceptions(cause);
        }

        set_up_supervisor_to_user_mode_transition()
            .expect("TRAP ERROR - CONTEXT NONE WHILE RETURNING TO USER MODE");

        unsafe {
            let return_to_user_mode_ptr: fn(usize) -> ! = transmute(TRAMPOLINE + TRAMPOLINE_OFFSET);
            return_to_user_mode_ptr((&raw const **trapframe).addr());
        }
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

pub fn handle_interrupts(cause: Scause) {
    if cause.cause() == Trap::Interrupt(Interrupt::SupervisorTimer as usize) {
        set_next_timer_interrupt(TIME_SLICE);
        yield_cpu();
    } else if cause.cause() == Trap::Interrupt(Interrupt::SupervisorExternal as usize) {
        let id = plic::claim();

        if id == UART_ID {
            uart::handle_interrupt();
        }

        if id != 0 {
            plic::complete(id);
        }
    }
}

pub fn handle_exceptions(cause: Scause) {
    if cause.cause() == Trap::Exception(Exception::UserEnvCall as usize) {
        syscall::handle();
    } else {
        panic!("{:?}", cause);
    }
}
