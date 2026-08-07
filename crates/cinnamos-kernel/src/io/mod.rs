pub mod serial;

/// A trait for writing bytes into a byte-oriented consumer.
pub trait Write {
    /// The type of error that should be returned when write operations failed.
    type Error;

    /// Writes each byte from `buf` into a byte consumer.
    ///
    /// Returns the number of bytes successfully written, or an [`Error`](Self::Error)
    /// if the write operation fails. If the function returns `Error`, then no bytes
    /// have been written into the consumer.
    ///
    /// A writer may choose to store bytes in an internal buffer instead, and only write
    /// when explicitly flushed. In that case, [`flush`](`Self::flush`) should be called
    /// after all bytes have been written.
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Flushes any buffered data into the consumer.
    ///
    /// Returns an [`Error`](Self::Error) if the flush operation fails.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// A trait for reading bytes from a byte-oriented producer.
pub trait Read {
    /// The type of error that should be returned when read operations failed.
    type Error;

    /// Reads as many bytes from a byte producer as possible, and stores them into `buf`.
    ///
    /// If the operation successfully reads `N` bytes, the bytes will be stored in
    /// `buf[0..N]` in the same order they're received, and the function will return
    /// `N`. Otherwise, the function returns an [`Error`](Self::Error). If the function
    /// returns an `Error`, then no bytes have been read and written to `buf`.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}
