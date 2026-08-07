use core::{
    num::NonZero,
    sync::atomic::{AtomicPtr, Ordering},
};

use riscv::register::{sie, sstatus};

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
/// [`InvalidInterruptSource`](InterruptError::InvalidInterruptSource).
/// - If no handler is registered for `irq`, this function returns
/// [`InterruptUnhandled`](InterruptError::InterruptUnhandled).
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
pub fn enable_interrupts() {
    let mut sie = sie::read();
    sie.set_stimer(true);
    sie.set_ssoft(true);
    sie.set_sext(true);
    unsafe { sie::write(sie) };
}

/// Encapsulates a saved IRQ state of a hart.
#[derive(Debug)]
pub struct IrqState {
    enabled: bool,
}

impl IrqState {
    /// Saves the current interrupt state and disables interrupts (if it is currently enabled).
    pub fn save_disable() -> Self {
        let sstatus = sstatus::read();
        unsafe { sstatus::clear_sie() };
        Self { enabled: sstatus.sie() }
    }

    /// Consumes and restores the previously saved interrupt state.
    pub fn restore(self) {
        if self.enabled {
            unsafe { sstatus::set_sie() };
        } else {
            unsafe { sstatus::clear_sie() };
        }
    }
}
