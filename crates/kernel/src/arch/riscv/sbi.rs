use core::sync::atomic::{AtomicBool, Ordering};
use crate::{print, println};

pub struct SbiCaps {
    timer: AtomicBool,
    debug_console: AtomicBool,
}

impl SbiCaps {
    #[inline(always)]
    pub fn has_timer(&self) -> bool {
        self.timer.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn has_debug_console(&self) -> bool {
        self.debug_console.load(Ordering::Relaxed)
    }
}

pub static SBI_CAPS: SbiCaps = SbiCaps {
    timer: AtomicBool::new(false),
    debug_console: AtomicBool::new(false),
};

pub fn init() {
    let time = sbi_rt::probe_extension(sbi_rt::Timer);
    let dbcn = sbi_rt::probe_extension(sbi_rt::Console);

    SBI_CAPS.timer.store(time.is_available(), Ordering::Relaxed);
    SBI_CAPS.debug_console.store(dbcn.is_available(), Ordering::Relaxed);

    print!("\nOpenSBI Extensions: ");
    print!("base");
    if time.is_available() { print!(",time"); }
    if dbcn.is_available() { print!(",dbcn"); }
    println!();
}
