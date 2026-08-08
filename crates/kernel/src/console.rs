use core::fmt::{self, Write};

use spin::MutexGuard;

use crate::{
    arch::{IrqDisabledSection, get_fallback_console},
    io::serial::SerialOutputWrite,
    sync::mutex_irq::MutexIrq,
};

/// A trait for a possibly flushable UTF-8-encoded data writer.
///
/// This trait should be implemented for a type that can act as a backend for [`Console`].
pub trait ConsoleWrite: Write + Sync {
    /// Flushes any buffered data to the output.
    fn flush(&mut self);
}

/// An abstraction of the kernel output console. Supports multiple console backends.
pub struct Console {
    serial: SerialOutputWrite,
}

impl Console {
    /// Creates a new console.
    #[inline]
    pub const fn new() -> Self {
        Self {
            serial: SerialOutputWrite,
        }
    }

    /// Writes a UTF-8-encoded string to the console output.
    ///
    /// Writes will be performed on all currently active [`ConsoleWrite`] backends.
    /// If none of the writes succeed, the function will write to the fallback console,
    /// which is always available (see [`get_fallback_console`]).
    #[inline]
    pub fn write(&mut self, s: &str) -> fmt::Result {
        if self.serial.write_str(s).is_err() {
            get_fallback_console().write_str(s)?;
        }
        Ok(())
    }

    /// Flushes all currently active [`ConsoleWrite`] backends.
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

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s)
    }
}

static CONSOLE: MutexIrq<Console> = MutexIrq::new(Console::new());

/// Acquires a lock on the global console.
#[inline]
pub fn lock<'ms>(ms: IrqDisabledSection<'ms>) -> MutexGuard<'ms, Console> {
    CONSOLE.lock(ms)
}
