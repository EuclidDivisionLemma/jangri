use core::ptr::{read_volatile, write_volatile};

pub const UART0: usize = 0x10000000;
pub const UART_TEXT_BUFFER_SIZE: usize = 1000;

pub const RBR: usize = UART0; // Receiver Buffer Register
pub const THR: usize = 0; // Transmitter Holding Register
pub const IER: usize = 1; // Interrupt Enable Register
pub const ISR: usize = 2; // Interrupt Status Register
pub const LCR: usize = 3; // Line Control Register
pub const LSR: usize = 5; // Line Status Register
pub const FCR: usize = 2; // FIFO Control Register
pub const DIVISOR_LATCH_HIGH: usize = 1; // Divisor Latch High Byte
pub const DIVISOR_LATCH_LOW: usize = 0; // Divisor Latch Low Byte

static mut TX_BUSY: bool = false;

pub fn write(offset: usize, value: u8) {
    unsafe {
        write_volatile((UART0 + offset) as *mut u8, value);
    }
}

pub fn read(offset: usize) -> u8 {
    unsafe { read_volatile((UART0 + offset) as *const u8) }
}

#[unsafe(no_mangle)]
pub fn initialise_uart() {
    write(IER, 0);

    write(LCR, 1 << 7);

    write(DIVISOR_LATCH_LOW, 0x03);

    write(DIVISOR_LATCH_HIGH, 0x00);

    write(LCR, 0);

    write(FCR, 0b111);

    write(IER, 0b11);
}

pub fn write_char(byte: u8) {
    while unsafe { TX_BUSY == true } {
        core::hint::spin_loop();
    }

    unsafe {
        TX_BUSY = true;
    }

    write(THR, byte);
}

#[unsafe(no_mangle)]
pub fn console_write(text: &str) {
    for byte in text.bytes() {
        if byte == '\n' as u8 {
            write_char('\n' as u8);
            write_char('\r' as u8);
        } else {
            write_char(byte);
        }
    }
}

pub fn handle_interrupt() {
    read(ISR);

    if read(LSR) & (1 << 5) != 0 {
        unsafe {
            TX_BUSY = false;
        }
    }
}
