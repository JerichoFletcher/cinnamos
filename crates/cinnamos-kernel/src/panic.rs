use core::{fmt::Write, panic::PanicInfo};

struct SbiWriter;

impl Write for SbiWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            sbi::legacy::console_putchar(c);
        }
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let msg = match info.location() {
        Some(loc) => format_args!(
            "Panic: at {}:{}: {}",
            loc.file(),
            loc.line(),
            info.message()
        ),
        None => format_args!("Panic: {}", info.message()),
    };

    if log::max_level() != log::LevelFilter::Off {
        log::error!("{msg}");
    } else {
        let _ = write!(SbiWriter, "{msg}");
    }
    loop {}
}
