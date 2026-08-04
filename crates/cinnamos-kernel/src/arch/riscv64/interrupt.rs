use core::{
    num::NonZero,
    sync::atomic::{AtomicPtr, Ordering},
};

use riscv::register::{sie, sstatus};

use crate::arch::device::plic::INTERRUPT_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    InvalidInterruptSource,
    InterruptUnhandled,
}

static INTERRUPT_HANDLERS: [AtomicPtr<()>; INTERRUPT_COUNT] =
    [const { AtomicPtr::null() }; INTERRUPT_COUNT];

pub fn register_irq_handler(source: NonZero<u16>, handler: fn()) -> Result<(), InterruptError> {
    let source = source.get();
    if (1..INTERRUPT_COUNT as u16).contains(&source) {
        INTERRUPT_HANDLERS[source as usize].store(handler as *mut (), Ordering::Release);
        Ok(())
    } else {
        Err(InterruptError::InvalidInterruptSource)
    }
}

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

pub fn enable_interrupts() {
    let mut sie = sie::read();
    sie.set_stimer(true);
    sie.set_ssoft(true);
    sie.set_sext(true);
    unsafe { sie::write(sie); }
}

#[derive(Debug)]
pub struct IrqState {
    enabled: bool,
}

impl IrqState {
    pub fn save_disable() -> Self {
        let sstatus = sstatus::read();
        unsafe { sstatus::clear_sie(); }
        Self { enabled: sstatus.sie() }
    }

    pub fn restore(self) {
        if self.enabled {
            unsafe { sstatus::set_sie(); }
        } else {
            unsafe { sstatus::clear_sie(); }
        }
    }
}
