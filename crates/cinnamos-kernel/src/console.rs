use core::fmt::{self, Write};

use crate::{device::uart::SerialWrite, sync::mutex_irqsave::MutexIrqSave};

pub trait ConsoleWrite: fmt::Write + Send {
    fn flush(&mut self);
}

pub struct Console {
    serial: SerialWrite,
}

impl Console {
    #[inline]
    pub const fn new() -> Self {
        Self {
            serial: SerialWrite::new(),
        }
    }

    #[inline]
    pub fn write(&mut self, s: &str) -> fmt::Result {
        self.serial.write_str(s)
    }

    #[inline]
    pub fn flush(&mut self) {
        self.serial.flush();
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

static CONSOLE: MutexIrqSave<Console> = MutexIrqSave::new(Console::new());

pub struct ConsoleWriter;

impl fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        CONSOLE.lock().write(s)
    }
}
