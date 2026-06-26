pub mod context;
pub mod trap;
pub mod timer;

fn enable_interrupts() {
    let mut sstatus = riscv::register::sstatus::read();
    sstatus.set_sie(true);
}

pub fn init() {
    trap::init();

    timer::schedule_timer();
    timer::enable_timer();

    enable_interrupts();
}
