use core::{marker::PhantomData, num::NonZero, ptr::NonNull};

use spin::{Once, RwLock, RwLockReadGuard, rwlock::RwLockWriteGuard};

use crate::hloc;

pub const INTERRUPT_COUNT: usize = 1024;
pub const MAX_PLIC_CONTEXT: usize = 15872;

const OFFSET_INTERRUPT_PRIORITY: usize  = 0x000000;
const OFFSET_INTERRUPT_ENABLE: usize    = 0x002000;
const OFFSET_INTERRUPT_CONTEXT: usize   = 0x200000;

const STRIDE_INTERRUPT_CONTEXT: usize   = 0x001000;

#[repr(C)]
struct PlicContext {
    priority_threshold: u32,
    irq_claim_complete: u32,
}

#[derive(Debug)]
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
            Self {
                base_addr,
                max_priority,
            }
        }
    }

    pub fn set_priority(&mut self, source: u16, priority: u32) {
        debug_assert!((1..INTERRUPT_COUNT as u16).contains(&source));
        unsafe {
            let ptr = self
                .base_addr
                .byte_add(OFFSET_INTERRUPT_PRIORITY)
                .add(source as usize);
            ptr.write_volatile(self.max_priority.min(priority));
        }
    }

    pub fn set_enabled(&mut self, source: u16, hid: usize, enabled: bool) {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        debug_assert!((1..INTERRUPT_COUNT as u16).contains(&source));
        unsafe {
            const CTX_WIDTH: usize = INTERRUPT_COUNT / 8;
            let ptr = self
                .base_addr
                .byte_add(OFFSET_INTERRUPT_ENABLE + Self::plic_ctx_id(hid) * CTX_WIDTH)
                .add(source as usize / 32);
            let shift = source % 32;

            let val = ptr.read_volatile();
            ptr.write_volatile(if enabled {
                val | (1u32 << shift)
            } else {
                val & !(1u32 << shift)
            });
        }
    }

    pub fn set_threshold(&mut self, hid: usize, threshold: u32) {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        unsafe {
            let ptr = self.plic_ctx(hid);
            (&raw mut (*ptr).priority_threshold).write_volatile(threshold);
        }
    }

    fn claim_irq(&self, hid: usize) -> u16 {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        debug_assert_eq!(
            hid,
            hloc::get().hid(),
            "attempting to claim IRQ for hart {} from hart {}",
            hloc::get().hid(),
            hid
        );
        unsafe {
            let ptr = self.plic_ctx(hid);
            (&raw mut (*ptr).irq_claim_complete).read_volatile() as u16
        }
    }

    fn complete_irq(&self, hid: usize, irq: NonZero<u16>) {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        debug_assert_eq!(
            hid,
            hloc::get().hid(),
            "attempting to complete IRQ for hart {} from hart {}",
            hloc::get().hid(),
            hid
        );
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
unsafe impl Sync for Plic {}

static PLIC: Once<RwLock<Plic>> = Once::new();

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
        get_plic().complete_irq(self.hid, self.irq_id);
    }
}

/// # Panic
/// Will panic if PLIC driver is not initialized.
pub fn get_plic<'a>() -> RwLockReadGuard<'a, Plic> {
    PLIC.get().expect("PLIC used before init").read()
}

/// # Panic
/// Will panic if PLIC driver is not initialized.
pub fn get_plic_mut<'a>() -> RwLockWriteGuard<'a, Plic> {
    PLIC.get().expect("PLIC used before init").write()
}

pub fn init(base_addr: NonNull<u8>) {
    PLIC.call_once(|| unsafe { RwLock::new(Plic::new(base_addr.as_ptr().cast())) });
}

pub fn claim_irq(hid: usize) -> Option<PlicIrqClaim> {
    let irq = get_plic().claim_irq(hid);
    Some(PlicIrqClaim {
        hid,
        irq_id: NonZero::new(irq)?,
        _no_send: PhantomData,
    })
}
