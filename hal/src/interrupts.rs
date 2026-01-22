pub trait InterruptHandling {
    unsafe fn enable_interrupts();
    fn disable_interrupts();
    fn set_next_timer_interrupt(time: usize);
    fn are_interrupts_enabled() -> bool;
}
