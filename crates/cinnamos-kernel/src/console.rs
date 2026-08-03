use core::fmt::{self, Write};

use crate::{device::uart::SerialWrite, sync::mutex_irqsave::{MutexIrqSave, MutexIrqSaveGuard}};

pub trait ConsoleWrite: fmt::Write + Sync {
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

    #[inline]
    pub fn lock<'a>() -> MutexIrqSaveGuard<'a, Console> {
        CONSOLE.lock()
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s)
    }
}

static CONSOLE: MutexIrqSave<Console> = MutexIrqSave::new(Console::new());
