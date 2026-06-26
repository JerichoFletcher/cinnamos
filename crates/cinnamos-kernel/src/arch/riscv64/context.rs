use riscv::register::sstatus::Sstatus;

#[repr(C)]
pub struct RiscvContext {
    pub regs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: *const u8,
}
