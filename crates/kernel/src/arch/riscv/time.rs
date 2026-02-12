use crate::arch::riscv::sbi::SBI_CAPS;
use crate::arch::time::Time;
use crate::cpu::local;
use crate::cpu::local::CpuLocal;

pub struct RiscvTime;

impl Time for RiscvTime {
    fn now() -> u64 {
        riscv::register::time::read64()
    }

    fn deadline() -> u64 {
        local::get().next_deadline()
    }

    fn set_deadline(t: u64) {
        if SBI_CAPS.has_timer() {
            sbi_rt::set_timer(t);
            local::get_mut().set_next_deadline(t);
        } else {
            panic!("Timer not supported");
        }
    }
}
