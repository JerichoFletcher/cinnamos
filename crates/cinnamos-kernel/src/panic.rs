use core::panic::PanicInfo;

use crate::{print, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("Panic: ");
    
    match info.location() {
        Some(loc) => println!("{}:{}", loc.file(), loc.line()),
        None => println!("?"),
    }

    println!("{}", info.message());
    loop {}
}
