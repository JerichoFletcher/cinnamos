use core::{fmt::Write, panic::PanicInfo};

use crate::arch;

struct SbiWrite;

impl Write for SbiWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            sbi::debug_console::write_byte(c).map_err(|_| core::fmt::Error)?;
        }
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let msg = match info.location() {
        Some(loc) => format_args!(
            "kernel panic at {}:{}: {}",
            loc.file(),
            loc.line(),
            info.message()
        ),
        None => format_args!("kernel panic: {}", info.message()),
    };

    if log::log_enabled!(log::Level::Error) {
        log::error!("{msg}");
    } else {
        let _ = writeln!(SbiWrite, "{msg}");
    }
    loop {
        arch::wait_for_interrupt();
    }
}
