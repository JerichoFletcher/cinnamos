#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!(crate::device::uart::Writer::new(), $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => ({ $crate::print!("\r\n") });
    ($fmt:expr) => ({
        $crate::print!(concat!($fmt, "\r\n"))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::print!(concat!($fmt, "\r\n"), $($arg)+)
    });
}
