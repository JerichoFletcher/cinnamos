use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match info.location() {
        Some(loc) => log::error!("Panic: at {}:{}: {}", loc.file(), loc.line(), info.message()),
        None => log::error!("Panic: {}", info.message()),
    }
    loop {}
}
