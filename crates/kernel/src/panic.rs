use core::panic::PanicInfo;
use crate::cpu::idle;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    idle();
}
