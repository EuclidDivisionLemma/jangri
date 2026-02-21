use core::arch::global_asm;
use hal::interrupts::InterruptHandling;

use crate::{ARCH, global_state::GlobalState, process::ProcessState};

#[repr(C)]
#[derive(Clone, Default)]
pub struct Context {
    pub ra: usize,
    pub sp: usize,

    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
}

unsafe extern "C" {
    /// Switches context from the old context to the new context.
    /// Called inside scheduler to switch from scheduler context to process context
    /// and called inside `switch_to_scheduler_context` to switch from process context
    /// to scheduler context.
    fn switch_context(old: usize, new: usize);
}

global_asm!(
    r#"
    .section .text.switch_context
    .global switch_context

    switch_context:
        sd ra, 0(a0)
        sd sp, 8(a0)

        sd s0, 16(a0)
        sd s1, 24(a0)
        sd s2, 32(a0)
        sd s3, 40(a0)
        sd s4, 48(a0)
        sd s5, 56(a0)
        sd s6, 64(a0)
        sd s7, 72(a0)
        sd s8, 80(a0)
        sd s9, 88(a0)
        sd s10, 96(a0)
        sd s11, 104(a0)

        ld ra, 0(a1)
        ld sp, 8(a1)
        ld s0, 16(a1)
        ld s1, 24(a1)
        ld s2, 32(a1)
        ld s3, 40(a1)
        ld s4, 48(a1)
        ld s5, 56(a1)
        ld s6, 64(a1)
        ld s7, 72(a1)
        ld s8, 80(a1)
        ld s9, 88(a1)
        ld s10, 96(a1)
        ld s11, 104(a1)

        ret
    "#
);

pub fn schedule(state: &GlobalState) -> ! {
    loop {
        let mut found = false;

        if let Some(pid) = state.find_ready_process() {
            let locked_process = state.get_process(pid).unwrap();
            let context;

            let mut process = locked_process.lock();

            context = &raw const process.context;
            process.process_state = ProcessState::Running;
            drop(process);

            let scheduler_context = &raw const state.scheduler_context as *mut Context;

            unsafe {
                state.set_current_process(locked_process.clone());
                switch_context(scheduler_context.addr(), context.addr());
                found = true;
            }
        }

        if !found {
            ARCH::wfi();
        }
    }
}

/// Switches from the current process context to the scheduler context.
pub fn switch_to_scheduler_context(state: &GlobalState) {
    let context;

    let process = state.get_current_process().unwrap();
    let process = process.lock();
    context = &raw const process.context;
    drop(process);

    let scheduler_context = &raw const state.scheduler_context as *mut Context;

    unsafe {
        switch_context(context.addr(), scheduler_context.addr());
    }
}
