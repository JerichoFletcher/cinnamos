pub mod cpu;
pub mod context;
pub mod trap;
pub mod sbi;

pub fn init() {
    trap::init();
}
