use core::panic::PanicInfo;

use crate::arch;

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

    log::error!("{msg}");
    loop {
        arch::wait_for_interrupt();
    }
}
