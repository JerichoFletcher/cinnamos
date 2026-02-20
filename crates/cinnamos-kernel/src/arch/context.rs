pub trait Context {
    fn pc(&self) -> *const u8;
    fn set_pc(&mut self, pc: *const u8);
    fn advance_pc(&mut self);
}
