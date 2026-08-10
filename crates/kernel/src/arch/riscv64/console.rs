use core::fmt::Write;

use crate::{
    arch::addr::{PAddr, VAddr},
    console::ConsoleWrite,
    mem,
};

struct SbiWrite;

impl Write for SbiWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let trns = mem::vms::translate_virt(VAddr::from_ptr(s.as_bytes()));
        let phys = if let Some((phys, _)) = trns {
            phys
        } else {
            PAddr::from_ptr(s.as_bytes()) - mem::vms::phys_to_kernel_dynslide()
        };
        let phys_ptr = core::ptr::slice_from_raw_parts_mut(phys.addr() as *mut _, s.len());
        let phys_s = sbi::PhysicalAddress::from_ptr(phys_ptr);

        // Safety: phys_s points to s as a byte slice
        unsafe { sbi::debug_console::write_ptr(phys_s).map_err(|_| core::fmt::Error)? };
        Ok(())
    }
}

impl ConsoleWrite for SbiWrite {
    fn flush(&mut self) {}
}

/// Retrieves a fallback [console writer](ConsoleWrite) that is always safe to use. Note that
/// returning a do-nothing writer is a valid implementation; the only guarantee that the fallback
/// writer upholds is that it is valid during the entire lifetime of the kernel from boot.
pub fn get_fallback_console() -> impl ConsoleWrite {
    SbiWrite
}
