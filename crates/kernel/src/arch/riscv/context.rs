use riscv::register::sstatus::Sstatus;
use crate::arch::context::Context;

#[repr(C)]
pub struct RiscvRawContext {
    pub regs: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
}

pub struct RiscvContext<'a> {
    raw: &'a mut RiscvRawContext,
}

impl<'a> RiscvContext<'a> {
    pub fn new(raw: &'a mut RiscvRawContext) -> RiscvContext<'a> {
        RiscvContext { raw }
    }

    fn sstatus(&self) -> Sstatus {
        Sstatus::from_bits(self.raw.sstatus)
    }

    fn set_sstatus(&mut self, sstatus: Sstatus) {
        self.raw.sstatus = sstatus.bits();
    }
}

impl Context for RiscvContext<'_> {
    fn pc(&self) -> usize {
        self.raw.sepc
    }

    fn set_pc(&mut self, pc: usize) {
        self.raw.sepc = pc;
    }

    fn interrupts_enabled(&self) -> bool {
        self.sstatus().sie()
    }

    fn set_interrupts_enabled(&mut self, enabled: bool) {
        let mut sstatus = self.sstatus();
        sstatus.set_sie(enabled);
        self.set_sstatus(sstatus);
    }
}
