// use core::sync::atomic::AtomicUsize;

// use alloc::sync::Arc;
// use hal::error::Result;
// use ringbuffer::RingBuffer;

// use crate::Mutex;
// use crate::error::Error;
// use crate::global_state::GlobalState;
// use alloc::vec;
// use alloc::vec::Vec;
// use ringbuffer::AllocRingBuffer;

// use crate::process::{self};

// pub const PIPE_SIZE: usize = 4096;

// static FILE_DESCRIPTOR: AtomicUsize = AtomicUsize::new(3);

// pub struct Pipe {
//     data: AllocRingBuffer<u8>,
//     pub read_end_open: bool,
//     pub write_end_open: bool,
//     pub read_offset: usize,
//     pub write_offset: usize,
//     pub reader: usize,
//     pub writer: usize,
// }

// pub fn allocate_pipe(state: &GlobalState) -> Arc<Mutex<Pipe>> {
//     let reader = FILE_DESCRIPTOR.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
//     let writer = FILE_DESCRIPTOR.fetch_add(1, core::sync::atomic::Ordering::AcqRel);

//     let pipe = Arc::new(Mutex::new(Pipe {
//         data: AllocRingBuffer::new(PIPE_SIZE),
//         read_end_open: true,
//         write_end_open: true,
//         read_offset: 0,
//         write_offset: 0,
//         reader,
//         writer,
//     }));

//     let mut pipes = state.pipes.write();
//     pipes.insert(reader, pipe.clone());
//     pipes.insert(writer, pipe.clone());

//     pipe
// }

// impl Pipe {
//     pub fn write(&mut self, state: &GlobalState, buffer: &[u8]) -> Result<()> {
//         if self.write_end_open == false {
//             bail!(Error::PipeWriterClosed);
//         }

//         if self.read_end_open == false {
//             bail!(Error::PipeReaderClosed);
//         }

//         for i in 0..buffer.len() {
//             while let Some(_) = self.data.enqueue(buffer[i])
//                 && self.read_end_open
//             {
//                 state.disable_interrupts();

//                 if let Some(process) = state.get_current_process() {
//                     let mut process = process.lock();
//                     process::wake_up(state, (&raw const self.read_offset).addr());

//                     process.sleep((&raw const self.write_offset).addr());
//                 }
//             }

//             self.write_offset = (self.write_offset + 1) % PIPE_SIZE;
//         }

//         process::wake_up(state, (&raw const self.read_offset).addr());

//         Ok(())
//     }

//     pub fn read(&mut self, state: &GlobalState, num_bytes: usize) -> Vec<u8> {
//         let mut bytes = vec![];

//         for _ in 0..num_bytes {
//             while self.write_end_open {
//                 match self.data.dequeue() {
//                     Some(v) => {
//                         bytes.push(v);
//                         self.read_offset = (self.read_offset + 1) % PIPE_SIZE;
//                         break;
//                     }

//                     None => (),
//                 }
//             }
//         }

//         process::wake_up(state, (&raw const self.write_offset).addr());

//         bytes
//     }
// }
