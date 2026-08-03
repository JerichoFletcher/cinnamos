use riscv::register::time;

const TIMER_INTERVAL: u64 = 512;

pub fn schedule_timer() {
    let time = time::read64();
    let _ = sbi::timer::set_timer(time + TIMER_INTERVAL);
}
