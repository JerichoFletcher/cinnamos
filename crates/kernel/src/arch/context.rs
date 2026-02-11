pub trait Context {
    fn pc(&self) -> usize;
    fn set_pc(&mut self, pc: usize);

    fn interrupts_enabled(&self) -> bool;
    fn set_interrupts_enabled(&mut self, enabled: bool);
}
