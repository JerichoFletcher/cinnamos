use crate::cpu::interrupt::InterruptMask;

#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::trap::RiscvTrap as TrapImpl;

pub trait Trap {
    fn set_interrupt_mask(mask: &InterruptMask);
}

#[inline(always)]
pub fn set_interrupt_mask(mask: &InterruptMask) {
    TrapImpl::set_interrupt_mask(mask);
}
