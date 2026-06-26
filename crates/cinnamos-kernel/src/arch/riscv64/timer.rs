use riscv::register::{sie, time};

const TIMER_INTERVAL: u64 = 1024;

pub fn enable_timer() {
    let mut sie = sie::read();
    sie.set_stimer(true);
    unsafe { sie::write(sie); }
}

pub fn schedule_timer() {
    let time = time::read64();
    let _ = sbi::timer::set_timer(time + TIMER_INTERVAL);
}
