use core::num::NonZero;

use alloc::boxed::Box;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::arch::IrqDisabledSection;

pub mod plic;

pub type InterruptSource = NonZero<usize>;

/// Denotes the priority at which interrupts are chosen.
///
/// Higher priority interrupts will mask lower priority ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterruptPriority {
    /// This interrupt is disabled.
    Disabled,
    /// This interrupt is considered low priority.
    Low,
    /// This interrupt is considered medium priority.
    Medium,
    /// This interrupt is considered high priority.
    /// No other interrupts should be raised while handling an interrupt with this priority level.
    High,
}

/// Denotes the minimum level of priority of interrupts that should be raised on an interrupt context.
///
/// Each value corresponds to the minimum [`InterruptPriority`] that is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterruptPriorityThreshold {
    /// The interrupt context allows all priority levels, starting from [`InterruptPriority::Low`].
    All,
    /// The interrupt context only allows a minimum level of [`InterruptPriority::Medium`].
    Medium,
    /// The interrupt context only allows a minimum level of [`InterruptPriority::High`].
    High,
    /// The interrupt context disables all interrupts.
    Disabled,
}

impl InterruptPriorityThreshold {
    /// Generates a threshold that masks interrupts with equal or lower level than `priority`.
    #[inline]
    pub const fn mask_from(priority: InterruptPriority) -> Self {
        match priority {
            InterruptPriority::Disabled => Self::All,
            InterruptPriority::Low => Self::Medium,
            InterruptPriority::Medium => Self::High,
            InterruptPriority::High => Self::Disabled,
        }
    }
}

/// An abstraction for any configurable interrupt controller.
pub trait InterruptController: Send + Sync {
    /// Gets the priority of an interrupt source.
    fn get_priority(&self, source: InterruptSource) -> InterruptPriority;

    /// Sets the priority of an interrupt source.
    ///
    /// Since this mutates a global interrupt configuration, this function requires exclusive
    /// access to the controller to prevent races by other harts.
    fn set_priority(
        &mut self,
        ms: IrqDisabledSection<'_>,
        source: InterruptSource,
        priority: InterruptPriority,
    );

    /// Gets the priority threshold for interrupts that can be taken for this hart.
    fn get_threshold(&self) -> InterruptPriorityThreshold;

    /// Sets the priority threshold for interrupts that can be taken for this hart.
    fn set_threshold(&self, ms: IrqDisabledSection<'_>, threshold: InterruptPriorityThreshold);

    /// Gets whether the interrupt source is enabled for this hart.
    fn get_enabled(&self, source: InterruptSource) -> bool;

    /// Sets whether the interrupt source is enabled for this hart.
    fn set_enabled(&self, ms: IrqDisabledSection<'_>, source: InterruptSource, enabled: bool);

    /// Attempts to claim an IRQ from the current hart context.
    ///
    /// If an IRQ is pending and successfully claimed, interrupts with a lower priority
    /// level than the one claimed will be masked, and `f` will be called.
    /// IRQ masking will be restored after `f` returns.
    ///
    /// # Safety
    /// `f` is allowed to re-enable interrupts, which is undefined behavior within IRQ-free
    /// sections. As such, callers must make sure that this function is not called from within
    /// an IRQ-free section.
    unsafe fn try_if_claim(&self, f: &dyn Fn(InterruptSource));
}

static INTERRUPT_CONTROLLER: RwLock<Option<Box<dyn InterruptController>>> = RwLock::new(None);

/// Acquires a shared read lock on the current [`InterruptController`].
#[expect(unused)]
pub fn get_controller_read<'ms>(
    _ms: IrqDisabledSection<'ms>,
) -> RwLockReadGuard<'ms, Option<Box<dyn InterruptController>>> {
    INTERRUPT_CONTROLLER.read()
}

/// Acquires an exclusive write lock on the current [`InterruptController`].
#[expect(unused)]
pub fn get_controller_write<'ms>(
    _ms: IrqDisabledSection<'ms>,
) -> RwLockWriteGuard<'ms, Option<Box<dyn InterruptController>>> {
    INTERRUPT_CONTROLLER.write()
}

/// Installs a kernel [`InterruptController`] driver.
///
/// Returns the previously installed driver, if any.
#[inline]
pub fn set_controller(
    _ms: IrqDisabledSection<'_>,
    ic: Box<dyn InterruptController>,
) -> Option<Box<dyn InterruptController>> {
    let mut g = INTERRUPT_CONTROLLER.write();
    g.replace(ic)
}

/// Attempts to claim a pending IRQ, and calls `f` if one exists and is enabled for the current hart.
///
/// # Safety
/// `f` is allowed to re-enable interrupts, which is undefined behavior within IRQ-free
/// sections. As such, callers must make sure that this function is not called from within
/// an IRQ-free section.
#[inline]
pub unsafe fn with_claim(f: impl Fn(InterruptSource)) {
    let g = INTERRUPT_CONTROLLER.read();
    if let Some(ic) = g.as_ref() {
        unsafe { ic.try_if_claim(&f) };
    }
}
