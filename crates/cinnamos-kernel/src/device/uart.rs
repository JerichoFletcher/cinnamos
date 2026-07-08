use core::ptr::NonNull;

use spin::Mutex;
use uart::{Data, Uart, address::MmioAddress};

struct SendUart(Uart<MmioAddress, Data>);

unsafe impl Send for SendUart {}

static UART: Mutex<Option<SendUart>> = Mutex::new(None);

pub fn init(base_addr: NonNull<u8>) {
    let drv = unsafe { <Uart<_, Data>>::new(MmioAddress::new(base_addr, 1)) };
    *UART.lock() = Some(SendUart(drv));
}

pub struct Writer;

impl Writer {
    pub fn new() -> Self {
        Self
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if let Some(drv) = UART.lock().as_mut() {
            for c in s.bytes() {
                if c == b'\n' {
                    drv.0.write_byte(b'\r');
                    drv.0.write_byte(b'\n');
                } else {
                    drv.0.write_byte(c);
                }
            }
        }
        Ok(())
    }
}
