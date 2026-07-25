use riscv::register::sstatus::Sstatus;

use crate::arch::VAddr;

#[repr(C)]
#[derive(Debug)]
pub struct Context {
    pub regs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: VAddr,
}
