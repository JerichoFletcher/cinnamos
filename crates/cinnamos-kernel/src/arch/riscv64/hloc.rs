#[inline]
pub unsafe fn load_boot_hart_local<T>(hloc: *const T) {
    unsafe {
        core::arch::asm!(
            "mv tp, {0}",
            in(reg) hloc,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// # Safety
/// This function can only be safely called after [load_boot_hart_local](load_boot_hart_local) with the matching type `T`.
#[inline]
pub unsafe fn hart_local<T>() -> &'static mut T {
    let ptr: *mut T;
    unsafe {
        core::arch::asm!(
            "mv {0}, tp",
            out(reg) ptr,
            options(nomem, nostack, preserves_flags)
        );
        &mut *ptr
    }
}
