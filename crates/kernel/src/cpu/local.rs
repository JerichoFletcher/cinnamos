use crate::arch::cpu::CpuLocalImpl;

pub static MAX_CPU: usize = 1;

pub trait CpuLocal {
    fn id(&self) -> usize;

    fn kernel_sp(&self) -> usize;
    fn set_kernel_sp(&mut self, sp: usize);

    fn next_deadline(&self) -> u64;
    fn set_next_deadline(&mut self, next_deadline: u64);
}

#[inline(always)]
pub fn get() -> &'static CpuLocalImpl {
    crate::arch::cpu::local()
}

#[inline(always)]
pub fn get_mut() -> &'static mut CpuLocalImpl {
    crate::arch::cpu::local_mut()
}
