use riscv::register::sstatus::Sstatus;

use crate::arch::VAddr;

#[repr(C)]
pub struct Context {
    pub regs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: VAddr,
}
