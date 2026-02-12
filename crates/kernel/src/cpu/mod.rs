pub mod local;
pub mod interrupt;

pub enum PrivMode {
    User,
    Supervisor,
}

#[inline(always)]
pub fn init(id: usize) {
    crate::arch::cpu::init(id);
}

#[inline(always)]
pub fn idle() -> ! {
    crate::arch::cpu::idle();
}
