mod trap;

pub mod context;
pub mod timer;
pub mod paddr;

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
