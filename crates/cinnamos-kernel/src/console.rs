use core::fmt::{self, Write as CoreWrite};

use crate::{
    arch::get_fallback_console,
    io::serial::SerialOutputWrite,
    sync::mutex_irqsave::{MutexIrqSave, MutexIrqSaveGuard},
};

pub trait ConsoleWrite: CoreWrite + Sync {
    fn flush(&mut self);
}

pub struct Console {
    serial: SerialOutputWrite,
}

impl Console {
    #[inline]
    pub const fn new() -> Self {
        Self {
            serial: SerialOutputWrite,
        }
    }

    #[inline]
    pub fn write(&mut self, s: &str) -> fmt::Result {
        if self.serial.write_str(s).is_err() {
            get_fallback_console().write_str(s)?;
        }
        Ok(())
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

impl CoreWrite for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s)
    }
}

static CONSOLE: MutexIrqSave<Console> = MutexIrqSave::new(Console::new());
