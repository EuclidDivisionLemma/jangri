#[repr(C)]
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("Pipe Error: Write end of pipe is closed")]
    PipeWriterClosed,

    #[error("Pipe Error: Read end of pipe is closed")]
    PipeReaderClosed,
}
