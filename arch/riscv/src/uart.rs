use core::{
    ptr::{read_volatile, write_volatile},
    sync::atomic::{AtomicBool, AtomicUsize},
};

use ringbuffer::{AllocRingBuffer, RingBuffer};

use crate::Mutex;

pub const UART0: usize = 0x10000000;

pub const RBR: usize = UART0; // Receiver Buffer Register
pub const THR: usize = 0; // Transmitter Holding Register
pub const IER: usize = 1; // Interrupt Enable Register
pub const ISR: usize = 2; // Interrupt Status Register
pub const LCR: usize = 3; // Line Control Register
pub const LSR: usize = 5; // Line Status Register
pub const FCR: usize = 2; // FIFO Control Register
pub const DIVISOR_LATCH_HIGH: usize = 1; // Divisor Latch High Byte
pub const DIVISOR_LATCH_LOW: usize = 0; // Divisor Latch Low Byte

static TX_BUSY: AtomicBool = AtomicBool::new(false);
static CURSOR_POSITION: AtomicUsize = AtomicUsize::new(0);

pub static INPUT_BUFFER: spin::Lazy<Mutex<AllocRingBuffer<u8>>> =
    spin::Lazy::new(|| Mutex::new(AllocRingBuffer::new(4096)));

pub static mut READ: bool = false;

pub fn write(offset: usize, value: u8) {
    unsafe {
        write_volatile((UART0 + offset) as *mut u8, value);
    }
}

pub fn read(offset: usize) -> u8 {
    unsafe { read_volatile((UART0 + offset) as *const u8) }
}

pub fn initialise_uart() {
    write(IER, 0);

    write(LCR, 1 << 7);

    write(DIVISOR_LATCH_LOW, 0x08);

    write(DIVISOR_LATCH_HIGH, 0x00);

    write(LCR, 0);

    write(FCR, 0b111);

    write(IER, 0b11);
}

pub fn write_char(byte: u8) {
    while TX_BUSY.load(core::sync::atomic::Ordering::Acquire) == true {
        core::hint::spin_loop();
    }

    TX_BUSY.store(true, core::sync::atomic::Ordering::Release);

    write(THR, byte);
}

pub fn write_char_waiting(byte: u8) {
    while read(LSR) & (1 << 5) == 0 {
        core::hint::spin_loop();
    }

    write(THR, byte);
}

pub fn console_write(text: &str) {
    for byte in text.bytes() {
        if byte == '\n' as u8 || byte == '\r' as u8 {
            write_char('\n' as u8);
            write_char('\r' as u8);
        } else if byte == 0x7f || byte == 0x08 {
            write_char_waiting(0x7f);
            write_char_waiting(' ' as u8);
            write_char_waiting(0x7f);
        } else {
            write_char(byte);
        }
    }
}

pub fn console_write_bytes(bytes: &[u8]) {
    for byte in bytes {
        if *byte == '\n' as u8 || *byte == '\r' as u8 {
            write_char('\n' as u8);
            write_char('\r' as u8);
        } else if *byte == 0x7f || *byte == 0x08 {
            write_char_waiting(0x7f);
            write_char_waiting(' ' as u8);
            write_char_waiting(0x7f);
        } else {
            write_char(*byte);
        }
    }
}

pub fn handle_interrupt() {
    read(ISR);

    if read(LSR) & (1 << 5) != 0 {
        TX_BUSY.store(false, core::sync::atomic::Ordering::Release);
    }

    {
        loop {
            match read_char() {
                Some(v) => {
                    if unsafe { READ } {
                        if v == '\n' as u8 || v == '\r' as u8 {
                            write_char_waiting('\n' as u8);
                            write_char_waiting('\r' as u8);

                            let mut input_buffer = INPUT_BUFFER.lock();
                            input_buffer.enqueue('\n' as u8);
                        } else if v == 0x7f || v == 0x08 {
                            if CURSOR_POSITION.load(core::sync::atomic::Ordering::Acquire) > 0 {
                                write_char_waiting(0x08);
                                write_char_waiting(' ' as u8);
                                write_char_waiting(0x08);

                                let mut input_buffer = INPUT_BUFFER.lock();
                                input_buffer.enqueue(v);

                                let _ = CURSOR_POSITION
                                    .fetch_sub(1, core::sync::atomic::Ordering::AcqRel);
                            }
                        } else {
                            write_char(v);
                            let _ =
                                CURSOR_POSITION.fetch_add(1, core::sync::atomic::Ordering::AcqRel);

                            let mut input_buffer = INPUT_BUFFER.lock();
                            input_buffer.enqueue(v as u8);
                        }
                    }
                }
                None => break,
            }
        }
    }
}

pub fn read_char() -> Option<u8> {
    if read(LSR) & 0b1 == 1 {
        Some(unsafe { read_volatile(RBR as *const u8) })
    } else {
        None
    }
}
