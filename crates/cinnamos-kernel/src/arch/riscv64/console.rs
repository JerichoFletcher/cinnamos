use core::fmt::Write;

use crate::{
    arch::VAddr,
    console::ConsoleWrite,
    mem,
};

struct SbiWrite;

impl Write for SbiWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let phys_s = VAddr::from_ptr(s.as_bytes())
            .addr()
            .wrapping_sub(mem::vms::phys_to_kernel_dynslide());
        let phys_ptr = core::ptr::slice_from_raw_parts_mut(phys_s as *mut u8, s.len());
        let phys_s = sbi::PhysicalAddress::from_ptr(phys_ptr);

        // Safety: phys_s points to s as a byte slice
        unsafe { sbi::debug_console::write_ptr(phys_s).map_err(|_| core::fmt::Error)? };
        Ok(())
    }
}

impl ConsoleWrite for SbiWrite {
    fn flush(&mut self) {}
}

pub fn get_fallback_console() -> impl ConsoleWrite {
    SbiWrite
}
