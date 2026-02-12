use crate::arch::cpu::local;
use crate::arch::time;
use crate::cpu::local::CpuLocal;

pub static TICK_INTERVAL: u64 = 1000;

pub fn init() {
    let now = time::now();
    time::set_deadline(now + TICK_INTERVAL);
}

pub fn schedule_next() {
    let mut next = local().next_deadline();
    while next <= time::now() {
        next += TICK_INTERVAL;
    }
    time::set_deadline(next);
}
