use core::panic::PanicInfo;

use crate::syscall::stdout;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    stdout(
        format_args!("{}", info.message().as_str().unwrap_or(""))
            .as_str()
            .unwrap_or(""),
    );
    loop {}
}
