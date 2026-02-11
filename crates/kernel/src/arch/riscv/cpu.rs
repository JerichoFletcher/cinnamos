use riscv::asm::wfi;
use crate::arch::cpu::Cpu;

pub struct RiscvCpu;

impl Cpu for RiscvCpu {
    #[inline(always)]
    fn idle() -> ! {
        loop { wfi(); }
    }
}
