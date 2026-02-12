use crate::cpu::PrivMode;

pub trait Context {
    fn pc(&self) -> usize;
    fn set_pc(&mut self, pc: usize);

    fn privilege(&self) -> PrivMode;
    fn set_privilege(&mut self, privilege: PrivMode);

    fn interrupts_enabled(&self) -> bool;
    fn set_interrupts_enabled(&mut self, enabled: bool);
}
