use core::panic::PanicInfo;

use alloc::format;

use crate::{drivers::uart::console_write, syscall::stdout};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    stdout(&format!("PANIC: {}", info.message()));
    loop {}
}
