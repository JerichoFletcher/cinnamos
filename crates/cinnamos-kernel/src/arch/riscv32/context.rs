use riscv::register::sstatus::Sstatus;

use crate::arch::context::Context;

#[repr(C)]
pub struct RiscvContext {
    pub regs: [usize; 32],
    pub sstatus: usize,
    pub sepc: *const u8,
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
    fn pc(&self) -> *const u8 {
        self.sepc
    }

    fn set_pc(&mut self, pc: *const u8) {
        self.sepc = pc
    }

    fn advance_pc(&mut self) {
        self.sepc = unsafe { self.sepc.add(size_of::<u32>()) };
    }
}
