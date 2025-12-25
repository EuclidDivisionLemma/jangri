use core::cell::Cell;
use core::cell::RefCell;
use ringbuffer::RingBuffer;

use crate::error::Error;
use crate::error::Result;
use alloc::vec;
use alloc::{rc::Rc, vec::Vec};
use ringbuffer::AllocRingBuffer;

use crate::file::{File, FileType};
use crate::process::{self, CURRENT_PROCESS};

// use crate::{
//     error::{Error, Result},
//     process::{self, CURRENT_PROCESS},
// };

pub const PIPE_SIZE: usize = 1024;

pub struct Pipe {
    data: RefCell<AllocRingBuffer<u8>>,
    read_end_open: Cell<bool>,
    write_end_open: Cell<bool>,
    read_offset: Cell<usize>,
    write_offset: Cell<usize>,
}

pub fn allocate_pipe(reader: &Rc<File>, writer: &Rc<File>) -> Rc<Pipe> {
    let pipe = Rc::new(Pipe {
        data: RefCell::new(AllocRingBuffer::new(PIPE_SIZE)),
        read_end_open: Cell::new(true),
        write_end_open: Cell::new(true),
        read_offset: Cell::new(0),
        write_offset: Cell::new(0),
    });

    reader.file_type.set(FileType::Pipe(pipe.clone()));
    reader.readable.set(true);
    reader.writeable.set(false);

    writer.file_type.set(FileType::Pipe(pipe.clone()));
    writer.readable.set(false);
    writer.writeable.set(true);

    pipe
}

impl Pipe {
    pub fn write(&self, buffer: &[u8]) -> Result<()> {
        if self.write_end_open.get() == false {
            return Err(Error::PipeWriterClosed);
        }

        for i in 0..buffer.len() {
            while let Some(_) = self.data.borrow_mut().enqueue(buffer[i])
                && self.read_end_open.get()
            {
                riscv::interrupt::supervisor::disable();

                if let Some(process) = unsafe { CURRENT_PROCESS.as_mut() } {
                    process::wake_up((&raw const self.read_offset).addr());
                    process.sleep((&raw const self.write_offset).addr());
                }

                unsafe {
                    riscv::interrupt::supervisor::enable();
                }
            }

            self.write_offset
                .set((self.write_offset.get() + 1) % PIPE_SIZE);
        }

        process::wake_up((&raw const self.read_offset).addr());

        Ok(())
    }

    pub fn read(&self, num_bytes: usize) -> Vec<u8> {
        let mut bytes = vec![];

        for _ in 0..num_bytes {
            while self.write_end_open.get() {
                match self.data.borrow_mut().dequeue() {
                    Some(v) => {
                        bytes.push(v);
                        self.read_offset
                            .set((self.read_offset.get() + 1) % PIPE_SIZE);
                        break;
                    }

                    None => (),
                }
            }
        }

        process::wake_up((&raw const self.write_offset).addr());

        bytes
    }

    pub fn close(&mut self) {
        if self.read_end_open.get() == true {
            self.read_end_open.set(false);
            process::wake_up((&raw const self.write_offset).addr());
        }

        if self.write_end_open.get() == true {
            self.write_end_open.set(false);
            process::wake_up((&raw const self.read_offset).addr());
        }
    }
}
