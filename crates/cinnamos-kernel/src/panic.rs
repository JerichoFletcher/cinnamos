use core::panic::PanicInfo;
use crate::{print, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("Panic: ");
    if let Some(location) = info.location() {
        println!("{}:{}: {}", location.file(), location.line(), info.message());
    } else {
        println!("?: {}", info.message());
    }
    crate::cpu::idle();
}
