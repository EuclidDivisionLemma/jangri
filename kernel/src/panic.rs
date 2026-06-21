use core::panic::PanicInfo;

use janglib::{print, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use alloc::format;

    println!("PANIC: {}", info.message());
    loop {}
}
