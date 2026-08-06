use core::sync::atomic::{AtomicU64, Ordering};

use riscv::register::time;

static TIMER_INTERVAL: AtomicU64 = AtomicU64::new(0);

pub fn init_timer(interval: usize) {
    TIMER_INTERVAL.store(interval as u64, Ordering::Release);
    schedule_timer();
}

pub fn schedule_timer() {
    let time = time::read64();
    let _ = sbi::timer::set_timer(time + TIMER_INTERVAL.load(Ordering::Acquire));
}
