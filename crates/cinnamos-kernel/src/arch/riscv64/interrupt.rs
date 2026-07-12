use core::{num::NonZero, sync::atomic::{AtomicPtr, Ordering}};

use crate::arch::device::plic::INTERRUPT_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    InvalidInterruptSource,
    InterruptUnhandled,
}

static INTERRUPT_HANDLERS: [AtomicPtr<()>; INTERRUPT_COUNT] = [const { AtomicPtr::null() }; INTERRUPT_COUNT];

pub fn register_irq_handler(source: NonZero<u16>, handler: fn()) -> Result<(), InterruptError> {
    let source = source.get();
    if (1..INTERRUPT_COUNT as u16).contains(&source) {
        unsafe {
            INTERRUPT_HANDLERS[source as usize].store(core::mem::transmute(handler), Ordering::Release);
            return Ok(())
        }
    }
    Err(InterruptError::InvalidInterruptSource)
}

pub fn dispatch_irq(irq: NonZero<u16>) -> Result<(), InterruptError> {
    let irq = irq.get();
    if (1..INTERRUPT_COUNT as u16).contains(&irq) {
        let ptr = INTERRUPT_HANDLERS[irq as usize].load(Ordering::Acquire);
        if !ptr.is_null() {
            unsafe {
                let handler: fn() = core::mem::transmute(ptr);
                handler();
                return Ok(())
            }
        } else {
            return Err(InterruptError::InterruptUnhandled)
        }
    }
    Err(InterruptError::InvalidInterruptSource)
}
