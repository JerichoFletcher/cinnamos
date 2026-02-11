use riscv::register::sstatus::Sstatus;
use crate::arch::context::Context;

#[repr(C)]
pub struct RiscvContext {
    pub regs: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
}

impl RiscvContext {
    fn sstatus(&self) -> Sstatus {
        Sstatus::from_bits(self.sstatus)
    }

    fn set_sstatus(&mut self, sstatus: Sstatus) {
        self.sstatus = sstatus.bits();
    }
}

impl Context for RiscvContext {
    fn pc(&self) -> usize {
        self.sepc
    }

    fn set_pc(&mut self, pc: usize) {
        self.sepc = pc;
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
