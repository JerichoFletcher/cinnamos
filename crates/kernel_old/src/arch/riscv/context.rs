use riscv::register::sstatus::{Sstatus, SPP};
use crate::arch::context::Context;
use crate::cpu::PrivMode;

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
    fn new(privilege: PrivMode) -> Self {
        let mut ctx = Self {
            regs: [0; 32],
            sstatus: 0,
            sepc: 0,
        };
        ctx.set_privilege(privilege);
        ctx
    }

    fn pc(&self) -> usize {
        self.sepc
    }

    fn set_pc(&mut self, pc: usize) {
        self.sepc = pc;
    }

    fn privilege(&self) -> PrivMode {
        match self.sstatus().spp() {
            SPP::User => PrivMode::User,
            SPP::Supervisor => PrivMode::Supervisor,
        }
    }

    fn set_privilege(&mut self, privilege: PrivMode) {
        let mut sstatus = self.sstatus();
        sstatus.set_spp(match privilege {
            PrivMode::User => SPP::User,
            PrivMode::Supervisor => SPP::Supervisor,
        });
        self.set_sstatus(sstatus);
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
