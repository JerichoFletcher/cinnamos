use crate::arch::riscv::sbi::SBI_CAPS;
use crate::arch::timer::Timer;

pub struct RiscvTimer;

impl Timer for RiscvTimer {
    fn now() -> u64 {
        riscv::register::time::read64()
    }

    fn set_deadline(t: u64) {
        if SBI_CAPS.has_timer() {
            sbi_rt::set_timer(t);
        } else {
            panic!("Timer not supported");
        }
    }
}
