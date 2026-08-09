use core::{
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use riscv::{interrupt::Interrupt, register::sstatus};

use crate::{
    arch::ic::{InterruptSource, plic::INTERRUPT_COUNT},
    hloc::get_critical_nesting,
};

/// Represents possible errors that may occur when handling an IRQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    /// The IRQ source has an invalid identifier.
    InvalidInterruptSource,
    /// No handler has been registered for this IRQ.
    InterruptUnhandled,
}

static INTERRUPT_HANDLERS: [AtomicPtr<()>; INTERRUPT_COUNT.get()] =
    [const { AtomicPtr::null() }; INTERRUPT_COUNT.get()];

/// Registers a function to be called to handle interrupts from a source.
///
/// # Errors
/// If the given `source` is an invalid interrupt source, this function returns
/// [`InvalidInterruptSource`](InterruptError::InvalidInterruptSource).
pub fn register_irq_handler(source: InterruptSource, handler: fn()) -> Result<(), InterruptError> {
    if (1..INTERRUPT_COUNT.get()).contains(&source.get()) {
        INTERRUPT_HANDLERS[source.get()].store(handler as *mut (), Ordering::Release);
        Ok(())
    } else {
        Err(InterruptError::InvalidInterruptSource)
    }
}

/// Dispatches an IRQ, potentially invoking a registered handler.
///
/// # Errors
/// - If the given `irq` is an invalid interrupt source, this function returns
///   [`InvalidInterruptSource`](InterruptError::InvalidInterruptSource).
/// - If no handler is registered for `irq`, this function returns
///   [`InterruptUnhandled`](InterruptError::InterruptUnhandled).
pub fn dispatch_irq(irq: InterruptSource) -> Result<(), InterruptError> {
    if (1..INTERRUPT_COUNT.get()).contains(&irq.get()) {
        let ptr = INTERRUPT_HANDLERS[irq.get()].load(Ordering::Acquire);
        if !ptr.is_null() {
            // Safety: ptr was cast from an fn()
            let handler: fn() = unsafe { core::mem::transmute(ptr) };
            handler();
            Ok(())
        } else {
            Err(InterruptError::InterruptUnhandled)
        }
    } else {
        Err(InterruptError::InvalidInterruptSource)
    }
}

/// Enables general interrupts, including timers, software interrupts, and external interrupts.
#[inline]
pub fn enable_interrupts() {
    interrupt_free(|_| {
        // Safety: All interrupts are enabled while global interrupt is disabled
        unsafe { riscv::interrupt::enable_interrupt(Interrupt::SupervisorSoft) };
        unsafe { riscv::interrupt::enable_interrupt(Interrupt::SupervisorTimer) };
        unsafe { riscv::interrupt::enable_interrupt(Interrupt::SupervisorExternal) };
    });
}

/// Disables interrupts and calls the closure with an IRQ-free token.
pub fn interrupt_free<'ms, T>(f: impl FnOnce(IrqDisabledSection<'ms>) -> T) -> T {
    struct State {
        enabled: bool,
    }
    impl Drop for State {
        fn drop(&mut self) {
            if self.enabled {
                unsafe { riscv::interrupt::enable() };
            }
        }
    }

    let sstatus = sstatus::read();
    riscv::interrupt::disable();
    let _g = State {
        enabled: sstatus.sie(),
    };
    f(IrqDisabledSection::new())
}

/// Token for marking an IRQ-masked section.
#[derive(Debug, Clone, Copy)]
pub struct IrqDisabledSection<'ms> {
    _life: PhantomData<&'ms ()>,
    _no_send_sync: PhantomData<*mut ()>,
}

impl<'ms> IrqDisabledSection<'ms> {
    #[inline]
    const fn new() -> Self {
        Self {
            _life: PhantomData,
            _no_send_sync: PhantomData,
        }
    }
}

/// Allows deriving an IRQ-free token from another type.
///
/// # Safety
/// This trait should only be implemented for token types that mark stronger critical sections.
/// In particular, the source critical section must also disable interrupts for the current hart.
pub unsafe trait MasksIrq {
    /// Derives an IRQ-free token from this token.
    fn as_irq_mask<'ms>(&self) -> IrqDisabledSection<'ms> {
        IrqDisabledSection::new()
    }
}

// Safety: The implementation for this critical section disables interrupts (see Critical)
unsafe impl<'cs> MasksIrq for critical_section::CriticalSection<'cs> {}

static CRITICAL_MUTEX: AtomicBool = AtomicBool::new(false);

struct Critical;
critical_section::set_impl!(Critical);

unsafe impl critical_section::Impl for Critical {
    unsafe fn acquire() -> critical_section::RawRestoreState {
        let sstatus = sstatus::read();
        riscv::interrupt::disable();

        let nesting = get_critical_nesting();
        if nesting.load(Ordering::Relaxed) == 0 {
            loop {
                if CRITICAL_MUTEX
                    .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }

                while CRITICAL_MUTEX.load(Ordering::Relaxed) {
                    core::hint::spin_loop();
                }
            }
        }
        nesting.fetch_add(1, Ordering::Relaxed);
        sstatus.sie()
    }

    unsafe fn release(restore_state: critical_section::RawRestoreState) {
        let nesting = get_critical_nesting();
        let prev_depth = nesting.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(prev_depth > 0);

        if prev_depth == 1 {
            CRITICAL_MUTEX.store(false, Ordering::Release);
            if restore_state {
                // Safety: This marks the end of the critical section; thus, re-enabling interrupts is safe
                unsafe { riscv::interrupt::enable() };
            }
        }
    }
}
