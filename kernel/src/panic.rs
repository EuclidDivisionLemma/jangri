use core::panic::PanicInfo;

use crate::syscall::stdout;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    stdout(
        format_args!("PANIC: {}", info.message().as_str().unwrap_or("PANIC"))
            .as_str()
            .unwrap_or("PANIC"),
    );
    loop {}
}
