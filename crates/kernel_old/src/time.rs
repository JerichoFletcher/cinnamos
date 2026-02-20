use crate::arch::cpu::local;
use crate::arch::time;
use crate::cpu::local::CpuLocal;
use crate::println;

pub const TICK_INTERVAL: u64 = 1000;

pub fn init() {
    if time::has_timer() {
        let now = time::now();
        time::set_deadline(now + TICK_INTERVAL);
        println!("Timer Interval        : {}", TICK_INTERVAL);
    } else {
        println!("Timer not supported; scheduler will not preempt");
    }
}

pub fn schedule_next() {
    if !time::has_timer() {
        panic!("Timer not supported");
    }
    let mut next = local().next_deadline();
    while next <= time::now() {
        next += TICK_INTERVAL;
    }
    time::set_deadline(next);
}
