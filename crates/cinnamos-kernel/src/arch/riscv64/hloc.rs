#[inline]
pub fn load_hart_local<T>(hloc: *const T) {
    unsafe {
        core::arch::asm!(
            "mv tp, {0}",
            in(reg) hloc as usize,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// # Safety
/// This function can only be safely called after [load_hart_local](load_hart_local) with the matching type `T`.
#[inline]
pub unsafe fn hart_local<T>() -> *mut T {
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
