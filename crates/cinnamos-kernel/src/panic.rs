use core::panic::PanicInfo;

use crate::{print, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("Panic: ");

    match info.location() {
        Some(loc) => print!("{}:{}", loc.file(), loc.line()),
        None => print!("?"),
    }

    println!(": {}", info.message());
    loop {}
}
