use core::fmt::Write;

use crate::{
    arch::VAddr,
    console::ConsoleWrite,
    mem,
};

struct SbiWrite;

impl Write for SbiWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match mem::vms::translate_virt(VAddr::from_ptr(s.as_bytes())) {
            Some((phys, _)) => {
                let phys_ptr = core::ptr::slice_from_raw_parts_mut(phys.addr() as *mut u8, s.len());
                let phys_s = sbi::PhysicalAddress::from_ptr(phys_ptr);

                // Safety: phys_s points to s as a byte slice
                unsafe { sbi::debug_console::write_ptr(phys_s).map_err(|_| core::fmt::Error)? };
                Ok(())
            }
            None => Err(core::fmt::Error)
        }
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
