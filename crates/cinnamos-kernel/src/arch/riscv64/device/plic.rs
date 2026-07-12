use core::{marker::PhantomData, num::NonZero, ptr::NonNull, u32};

use spin::Mutex;

pub const INTERRUPT_COUNT: usize = 1024;
pub const MAX_PLIC_CONTEXT: usize = 15872;

const OFFSET_INTERRUPT_PRIORITY: usize  = 0x000000;
const OFFSET_INTERRUPT_ENABLE: usize    = 0x002000;
const OFFSET_INTERRUPT_CONTEXT: usize   = 0x200000;

const STRIDE_INTERRUPT_CONTEXT: usize   = 0x001000;

#[derive(Debug)]
struct PlicContext {
    priority_threshold: u32,
    irq_claim_complete: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Plic {
    base_addr: *mut u32,
    max_priority: u32,
}

impl Plic {
    /// # Safety
    /// `base_addr` must be the base address of an existing PLIC memory-mapped region.
    unsafe fn new(base_addr: *mut u32) -> Self {
        unsafe {
            let probe = base_addr.byte_add(OFFSET_INTERRUPT_PRIORITY).add(1);
            let prev = probe.read_volatile();
            probe.write_volatile(u32::MAX);
            let max_priority = probe.read_volatile();
            probe.write_volatile(prev);
            Self { base_addr, max_priority }
        }
    }

    pub fn set_priority(&self, source: u16, priority: u32) {
        debug_assert!((1..INTERRUPT_COUNT as u16).contains(&source));
        unsafe {
            let ptr = self.base_addr.byte_add(OFFSET_INTERRUPT_PRIORITY).add(source as usize);
            ptr.write_volatile(self.max_priority.min(priority));
        }
    }

    pub fn set_enabled(&self, source: u16, hid: usize, enabled: bool) {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        debug_assert!((1..INTERRUPT_COUNT as u16).contains(&source));
        unsafe {
            const CTX_WIDTH: usize = INTERRUPT_COUNT as usize / 8;
            let ptr = self.base_addr.byte_add(OFFSET_INTERRUPT_ENABLE + Self::plic_ctx_id(hid) * CTX_WIDTH).add(source as usize / 32);
            let shift = source % 32;

            let val = ptr.read_volatile();
            ptr.write_volatile(if enabled { val | (1u32 << shift) } else { val & !(1u32 << shift) });
        }
    }

    pub fn set_threshold(&self, hid: usize, threshold: u32) {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        unsafe {
            let ptr = self.plic_ctx(hid);
            (&raw mut (*ptr).priority_threshold).write_volatile(threshold);
        }
    }

    fn claim_irq(&self, hid: usize) -> u16 {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        unsafe {
            let ptr = self.plic_ctx(hid);
            return (&raw mut (*ptr).irq_claim_complete).read_volatile() as u16
        }
    }

    fn complete_irq(&self, hid: usize, irq: NonZero<u16>) {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        unsafe {
            let ptr = self.plic_ctx(hid);
            (&raw mut (*ptr).irq_claim_complete).write_volatile(irq.get() as u32);
        }
    }
    
    const fn plic_ctx(&self, hid: usize) -> *mut PlicContext {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        let off = OFFSET_INTERRUPT_CONTEXT + Self::plic_ctx_id(hid) * STRIDE_INTERRUPT_CONTEXT;
        unsafe { self.base_addr.byte_add(off).cast() }
    }

    const fn plic_ctx_id(hid: usize) -> usize {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        2 * hid + 1
    }
}

unsafe impl Send for Plic {}

static PLIC: Mutex<Option<Plic>> = Mutex::new(None);

#[derive(Debug)]
pub struct PlicIrqClaim {
    hid: usize,
    irq_id: NonZero<u16>,
    _no_send: PhantomData<*const ()>,
}

impl PlicIrqClaim {
    pub const fn irq_id(&self) -> NonZero<u16> {
        self.irq_id
    }
}

impl Drop for PlicIrqClaim {
    fn drop(&mut self) {
        let _ = acquire(|plic| plic.complete_irq(self.hid, self.irq_id));
    }
}

pub fn init(base_addr: NonNull<u8>) {
    let drv = unsafe { Plic::new(base_addr.as_ptr().cast()) };
    *PLIC.lock() = Some(drv);
}

pub fn acquire<T>(f: impl FnOnce(&Plic) -> T) -> Result<T, ()> {
    let guard = PLIC.lock();
    guard.as_ref().ok_or(()).map(|drv| f(drv))
}

pub fn claim_irq(hid: usize) -> Option<PlicIrqClaim> {
    let irq = acquire(|plic| plic.claim_irq(hid)).ok()?;
    Some(PlicIrqClaim { hid, irq_id: NonZero::new(irq)?, _no_send: PhantomData })
}
