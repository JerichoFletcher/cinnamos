#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!(crate::device::driver::uart::UartWrite::new(), $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => ({ $crate::print!("\n") });
    ($fmt:expr) => ({
        $crate::print!(concat!($fmt, "\n"))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::print!(concat!($fmt, "\n"), $($arg)+)
    });
}
