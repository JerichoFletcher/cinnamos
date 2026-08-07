use uart::*;

use crate::io::{Read, Write};

/// An [`io::Read`](`Read`) reader for reading bytes out of a memory-mapped UART device. 
pub struct UartReceiveRead<'a>(&'a mut Uart<address::MmioAddress, Data>);

impl<'a> UartReceiveRead<'a> {
    /// Creates a reader on the given [`Uart`] driver.
    pub const fn new(drv: &'a mut Uart<address::MmioAddress, Data>) -> Self {
        Self(drv)
    }
}

impl Read for UartReceiveRead<'_> {
    type Error = ();

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut i = 0;
        while i < buf.len()
            && self
                .0
                .read_line_status()
                .contains(LineStatus::DATA_AVAILABLE)
        {
            buf[i] = self.0.read_byte();
            i += 1
        }
        Ok(i)
    }
}

/// An [`io::Write`](`Write`) writer for writing bytes into a memory-mapped UART device.
pub struct UartTransmitWrite<'a>(&'a mut Uart<address::MmioAddress, Data>);

impl<'a> UartTransmitWrite<'a> {
    /// Creates a writer on the given [`Uart`] driver.
    pub const fn new(drv: &'a mut Uart<address::MmioAddress, Data>) -> Self {
        Self(drv)
    }
}

impl Write for UartTransmitWrite<'_> {
    type Error = ();

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut i = 0;
        while i < buf.len() && self.0.read_line_status().contains(LineStatus::THR_EMPTY) {
            self.0.write_byte(buf[i]);
            i += 1;
        }
        Ok(i)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
