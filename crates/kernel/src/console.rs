use core::fmt::{Result, Error, Write};

pub struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> Result {
        match crate::arch::console::putstr(s) {
            Ok(()) => Ok(()),
            Err(()) => Err(Error),
        }
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)+) => ({
        use core::fmt::Write;
        let _ = write!($crate::console::ConsoleWriter, $($arg)+);
    })
}

#[macro_export]
macro_rules! println {
    () => ({
        $crate::print!("\n")
    });
    ($fmt:expr) => ({
        $crate::print!(concat!($fmt, "\n"))
    });
    ($fmt:expr, $($args:tt)+) => ({
        $crate::print!(concat!($fmt, "\n"), $($args)+)
    });
}
