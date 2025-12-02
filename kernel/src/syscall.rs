use core::arch::asm;

pub fn stdout<'a>(text: &'a str) {
    let chars = text.as_bytes();
    unsafe {
        for char in chars {
            asm!("li a7, 0x4442434E",
            "li a6, 2",
            "mv a0, {}",
            "ecall",
            in(reg) *char);
        }
    }
}
