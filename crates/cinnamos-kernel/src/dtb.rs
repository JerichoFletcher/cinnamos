use core::ptr::NonNull;

use fdt::Fdt;

pub struct MemRegion {
    pub base: NonNull<u8>,
    pub size: Option<usize>,
}

pub fn find_compatible_region(dtb: &Fdt, compat: &[&str]) -> Option<MemRegion> {
    let nodes = dtb.find_compatible(compat)?;
    let reg = nodes.reg()?.next()?;
    MemRegion { base: unsafe { NonNull::new_unchecked(reg.starting_address.cast_mut()) }, size: reg.size }.into()
}
