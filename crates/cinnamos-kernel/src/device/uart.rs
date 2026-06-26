use core::ptr::NonNull;

use spin::Mutex;
use uart::{address::MmioAddress, writer::UartWriter};

struct SendUart(UartWriter<MmioAddress>);

unsafe impl Send for SendUart {}

static UART: Mutex<Option<SendUart>> = Mutex::new(None);

pub fn init(base_addr: NonNull<u8>) {
    let writer = unsafe { UartWriter::new(MmioAddress::new(base_addr, 1), true) };
    *UART.lock() = Some(SendUart(writer));
}

pub struct Writer;

impl Writer {
    pub fn new() -> Self {
        Self
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if let Some(writer) = UART.lock().as_mut() {
            writer.0.write_str(s).ok();
        }
        Ok(())
    }
}
