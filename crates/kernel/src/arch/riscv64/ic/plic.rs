use core::ptr::NonNull;

use crate::{arch::{self, IrqDisabledSection}, hloc};

use super::*;

/// Corresponds to a PLIC context register.
#[repr(C)]
struct PlicContext {
    priority_threshold: u32,
    irq_claim_complete: u32,
}

/// The maximum number of interrupt numbers supported by the PLIC specification.
pub const INTERRUPT_COUNT: InterruptSource = NonZero::new(1024).unwrap();
/// The maximum number of PLIC contexts supported by the PLIC specification.
pub const MAX_PLIC_CONTEXT: usize = 15872;

const OFFSET_INTERRUPT_PRIORITY: usize  = 0x000000;
const OFFSET_INTERRUPT_ENABLE: usize    = 0x002000;
const OFFSET_INTERRUPT_CONTEXT: usize   = 0x200000;
const STRIDE_INTERRUPT_CONTEXT: usize   = 0x001000;

/// A memory-mapped PLIC driver.
#[derive(Debug)]
pub struct Plic {
    base_addr: NonNull<u8>,
    max_priority_num: u32,
}

impl Plic {
    /// Creates a memory-mapped PLIC driver.
    ///
    /// # Safety
    /// `base_addr` must be the base address of an existing PLIC memory-mapped region.
    pub unsafe fn new(base_addr: NonNull<u8>) -> Self {
        unsafe {
            let probe = base_addr
                .add(OFFSET_INTERRUPT_PRIORITY)
                .cast::<u32>()
                .add(1);
            let prev = probe.read_volatile();
            probe.write_volatile(u32::MAX);
            let max_priority_num = probe.read_volatile();
            probe.write_volatile(prev);
            Self {
                base_addr,
                max_priority_num,
            }
        }
    }

    #[inline]
    const fn priority_from_num(&self, num: u32) -> Option<InterruptPriority> {
        match num {
            0 => Some(InterruptPriority::Disabled),
            1 => Some(InterruptPriority::Low),
            _ => {
                if num == self.max_priority_num.div_ceil(2) {
                    Some(InterruptPriority::Medium)
                } else if num == self.max_priority_num {
                    Some(InterruptPriority::High)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    const fn to_priority_num(&self, priority: InterruptPriority) -> u32 {
        match priority {
            InterruptPriority::Disabled => 0,
            InterruptPriority::Low      => 1,
            InterruptPriority::Medium   => self.max_priority_num.div_ceil(2),
            InterruptPriority::High     => self.max_priority_num,
        }
    }

    #[inline]
    const fn threshold_from_num(&self, num: u32) -> Option<InterruptPriorityThreshold> {
        match num {
            0 => Some(InterruptPriorityThreshold::All),
            _ => {
                if num == self.to_priority_num(InterruptPriority::Low) {
                    Some(InterruptPriorityThreshold::Medium)
                } else if num == self.to_priority_num(InterruptPriority::Medium) {
                    Some(InterruptPriorityThreshold::High)
                } else if num == self.to_priority_num(InterruptPriority::High) {
                    Some(InterruptPriorityThreshold::Disabled)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    const fn to_threshold_num(&self, threshold: InterruptPriorityThreshold) -> u32 {
        match threshold {
            InterruptPriorityThreshold::All         => 0,
            InterruptPriorityThreshold::Medium      => self.to_priority_num(InterruptPriority::Low),
            InterruptPriorityThreshold::High        => self.to_priority_num(InterruptPriority::Medium),
            InterruptPriorityThreshold::Disabled    => self.to_priority_num(InterruptPriority::High),
        }
    }

    #[inline]
    const fn plic_ctx(&self, hid: usize) -> NonNull<PlicContext> {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        let off = OFFSET_INTERRUPT_CONTEXT + Self::plic_ctx_id(hid) * STRIDE_INTERRUPT_CONTEXT;
        unsafe { self.base_addr.add(off).cast() }
    }

    #[inline]
    const fn plic_ctx_id(hid: usize) -> usize {
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));
        2 * hid + 1
    }
}

impl InterruptController for Plic {
    fn get_priority(&self, source: InterruptSource) -> InterruptPriority {
        debug_assert!((1..INTERRUPT_COUNT.get()).contains(&source.get()));
        let ptr = unsafe {
            self
                .base_addr
                .add(OFFSET_INTERRUPT_PRIORITY)
                .cast::<u32>()
                .add(source.get())
        };
        let num = unsafe { ptr.read_volatile() };
        self.priority_from_num(num).expect("invalid priority number")
    }

    fn set_priority(&mut self, _ms: IrqDisabledSection<'_>, source: InterruptSource, priority: InterruptPriority) {
        debug_assert!((1..INTERRUPT_COUNT.get()).contains(&source.get()));
        let priority = Ord::min(
            self.to_priority_num(priority),
            self.max_priority_num,
        );

        let ptr = unsafe {
            self
                .base_addr
                .add(OFFSET_INTERRUPT_PRIORITY)
                .cast::<u32>()
                .add(source.get())
        };
        unsafe { ptr.write_volatile(priority) };
    }

    fn get_threshold(&self) -> InterruptPriorityThreshold {
        let hid = hloc::get_hid();

        // Safety: This context is exclusively owned by the current hart
        let ctx = unsafe { self.plic_ctx(hid).as_mut() };
        self.threshold_from_num(ctx.priority_threshold).expect("invalid threshold number")
    }

    fn set_threshold(&self, ms: IrqDisabledSection<'_>, threshold: InterruptPriorityThreshold) {
        let hloc = hloc::borrow(ms);
        let hid = hloc.hid();

        // Safety: This context is exclusively owned by the current hart
        let ctx = unsafe { self.plic_ctx(hid).as_mut() };
        ctx.priority_threshold = self.to_threshold_num(threshold);
    }

    fn get_enabled(&self, source: InterruptSource) -> bool {
        debug_assert!((1..INTERRUPT_COUNT.get()).contains(&source.get()));
        let hid = hloc::get_hid();
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));

        const CTX_WIDTH: usize = INTERRUPT_COUNT.get() / 8;
        let ptr = unsafe {
            self
                .base_addr
                .add(OFFSET_INTERRUPT_ENABLE + Self::plic_ctx_id(hid) * CTX_WIDTH)
                .cast::<u32>()
                .add(source.get() / 32)
        };
        let shift = source.get() % 32;
        let val = unsafe { ptr.read_volatile() };
        val & (1u32 << shift) != 0
    }

    fn set_enabled(&self, ms: IrqDisabledSection<'_>, source: InterruptSource, enabled: bool) {
        debug_assert!((1..INTERRUPT_COUNT.get()).contains(&source.get()));
        let hloc = hloc::borrow(ms);
        let hid = hloc.hid();
        debug_assert!(hid < (MAX_PLIC_CONTEXT / 2));

        const CTX_WIDTH: usize = INTERRUPT_COUNT.get() / 8;
        let ptr = unsafe {
            self
                .base_addr
                .add(OFFSET_INTERRUPT_ENABLE + Self::plic_ctx_id(hid) * CTX_WIDTH)
                .cast::<u32>()
                .add(source.get() / 32)
        };
        let shift = source.get() % 32;
        let old_val = unsafe { ptr.read_volatile() };
        let new_val = if enabled {
            old_val | (1u32 << shift)
        } else {
            old_val & !(1u32 << shift)
        };
        unsafe { ptr.write_volatile(new_val) };
    }

    unsafe fn try_if_claim(&self, f: &dyn Fn(InterruptSource)) {
        let claim = arch::interrupt_free(|ms| {
            let hloc = hloc::borrow(ms);
            let hid = hloc.hid();
            let ctx = self.plic_ctx(hid).as_ptr();

            // Safety: This context is exclusively owned by the current hart
            let irq_claim_ptr = unsafe { (&raw const (*ctx).irq_claim_complete) };
            let prev_threshold = self.get_threshold();
            InterruptSource::new(unsafe { irq_claim_ptr.read_volatile() } as _)
                .map(|irq_id| {
                    arch::interrupt_free(|ms| {
                        self.set_threshold(
                            ms,
                            InterruptPriorityThreshold::mask_from(self.get_priority(irq_id)),
                        );
                    });
                    (irq_id, prev_threshold)
                })
        });

        if let Some((irq_id, prev_threshold)) = claim {
            // Safety: Currently not in an IRQ-free section
            f(irq_id);

            arch::interrupt_free(|ms| {
                let hloc = hloc::borrow(ms);
                let hid = hloc.hid();
                let ctx = self.plic_ctx(hid).as_ptr();
    
                // Safety: Thisc context is exclusively owned by the current hart
                let irq_complete_ptr = unsafe { (&raw mut (*ctx).irq_claim_complete) };
                unsafe { irq_complete_ptr.write_volatile(irq_id.get() as _) };

                arch::interrupt_free(|ms| {
                    self.set_threshold(ms, prev_threshold);
                });
            });
        }
    }
}

// Safety: base_addr is mapped to a PLIC MMIO region, which is valid for all harts
unsafe impl Send for Plic {}
// Safety: All &self accesses are required to be hart-exclusive
// In particular, hid must be equal to the current hart's ID
unsafe impl Sync for Plic {}
