use sbi::SbiError;

use crate::{
    arch::VAddr,
    mem::{self, PhysFrameAlloc, physalloc::FrameAlloc},
};

/// Reflects errors that might occur while attempting to start a hart.
#[derive(Debug, PartialEq, Eq)]
pub enum HartStartError {
    /// The entry address cannot be translated to a physical address within the kernel address space.
    UnmappedEntry,
    /// The entry address translates to an invalid physical address.
    InvalidAddress,
    /// The given hart ID is invalid or cannot start in s-mode.
    InvalidHartId,
    /// The hart is already running.
    AlreadyStarted,
    /// The hart cannot be started for an unknown reason.
    Unknown,
}

/// Starts a hart on the given entry point.
///
/// # Safety
/// `entry` has to be a valid entry point in kernel space.
#[inline]
pub unsafe fn start_hart(
    hid: usize,
    entry: *const (),
    stack: FrameAlloc,
) -> Result<(), HartStartError> {
    let Some((entry_pa, _)) = mem::vms::translate_virt(VAddr::from_ptr(entry)) else {
        return Err(HartStartError::UnmappedEntry);
    };
    unsafe { sbi::hsm::hart_start(hid, entry_pa.into(), stack.end_addr().addr()) }
        .map(|_| core::mem::forget(stack))
        .map_err(|e| match e {
            SbiError::INVALID_ADDRESS => HartStartError::InvalidAddress,
            SbiError::INVALID_PARAMETER => HartStartError::InvalidHartId,
            SbiError::ALREADY_AVAILABLE => HartStartError::AlreadyStarted,
            _ => HartStartError::Unknown,
        })
}
