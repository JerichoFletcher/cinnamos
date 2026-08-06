pub mod serial;

pub trait Write {
    type Error;

    /// Returns the number of bytes successfully written.
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error>;
}

pub trait Read {
    type Error;

    /// Returns the number of bytes successfully read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}
