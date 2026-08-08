use core::{
    num::NonZero,
    sync::atomic::{AtomicPtr, Ordering},
};

use riscv::{interrupt::Interrupt, register::sstatus};

use crate::arch::device::plic::INTERRUPT_COUNT;

/// Represents possible errors that may occur when handling an IRQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    /// The IRQ source has an invalid identifier.
    InvalidInterruptSource,
    /// No handler has been registered for this IRQ.
    InterruptUnhandled,
}

static INTERRUPT_HANDLERS: [AtomicPtr<()>; INTERRUPT_COUNT] =
    [const { AtomicPtr::null() }; INTERRUPT_COUNT];

/// Registers a function to be called to handle interrupts from a source.
///
/// # Errors
/// If the given `source` is an invalid interrupt source, this function returns
/// [`InvalidInterruptSource`](InterruptError::InvalidInterruptSource).
pub fn register_irq_handler(source: NonZero<u16>, handler: fn()) -> Result<(), InterruptError> {
    let source = source.get();
    if (1..INTERRUPT_COUNT as u16).contains(&source) {
        INTERRUPT_HANDLERS[source as usize].store(handler as *mut (), Ordering::Release);
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
pub fn dispatch_irq(irq: NonZero<u16>) -> Result<(), InterruptError> {
    let irq = irq.get();
    if (1..INTERRUPT_COUNT as u16).contains(&irq) {
        let ptr = INTERRUPT_HANDLERS[irq as usize].load(Ordering::Acquire);
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
    interrupt_free(|| {
        // Safety: All interrupts are enabled while global interrupt is disabled
        unsafe { riscv::interrupt::enable_interrupt(Interrupt::SupervisorSoft) };
        unsafe { riscv::interrupt::enable_interrupt(Interrupt::SupervisorTimer) };
        unsafe { riscv::interrupt::enable_interrupt(Interrupt::SupervisorExternal) };
    })
}

/// Executes a closure with interrupts disabled for the current hart.
///
/// This effectively runs `f` as a single-hart critical section, in that execution will not be
/// preempted by interrupts. This does not prevent other harts from entering the same section; other
/// synchronization strategies (such as mutexes) should be used in that case.
#[inline]
pub fn interrupt_free<T>(f: impl FnOnce() -> T) -> T {
    let irq = IrqState::save_disable();
    let val = f();
    irq.restore();
    val
}

/// Encapsulates a saved IRQ state of a hart.
#[derive(Debug)]
pub struct IrqState {
    enabled: bool,
}

impl IrqState {
    /// Saves the current interrupt state and disables interrupts (if it is currently enabled).
    ///
    /// When paired with a [`restore`](Self::restore), this creates a single-hart critical section
    /// where execution cannot be preempted by interrupts. Re-enabling interrupts within a critical
    /// section is therefore unsafe.
    #[inline]
    pub fn save_disable() -> Self {
        let sstatus = sstatus::read();
        riscv::interrupt::disable();
        Self { enabled: sstatus.sie() }
    }

    /// Consumes and restores the previously saved interrupt state.
    ///
    /// When paired with a [`save_disable`](Self::save_disable), this creates a single-hart critical
    /// section where execution cannot be preempted by interrupts. Re-enabling interrupts within a
    /// critical section is unsafe and can lead to breaking the critical section.
    #[inline]
    pub fn restore(self) {
        if self.enabled {
            // Safety: This marks the end of the critical section; thus, re-enabling interrupts is safe
            unsafe { riscv::interrupt::enable() };
        }
    }
}
