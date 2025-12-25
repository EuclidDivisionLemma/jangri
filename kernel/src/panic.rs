use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::syscall::stdout;
    use alloc::format;

    stdout(&format!("PANIC: {}", info.message()));
    loop {}
}
