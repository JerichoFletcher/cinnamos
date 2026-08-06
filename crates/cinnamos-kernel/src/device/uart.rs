use uart::*;

use crate::io::{Read, Write};

pub struct UartReceiveRead<'a>(&'a mut Uart<address::MmioAddress, Data>);

impl<'a> UartReceiveRead<'a> {
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

pub struct UartTransmitWrite<'a>(&'a mut Uart<address::MmioAddress, Data>);

impl<'a> UartTransmitWrite<'a> {
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
