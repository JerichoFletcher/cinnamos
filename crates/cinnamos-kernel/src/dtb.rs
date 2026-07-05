use fdt::Fdt;

use crate::mem::MemoryRegion;

pub fn find_compatible_region(dtb: &Fdt, compat: &[&str]) -> Option<MemoryRegion> {
    let nodes = dtb.find_compatible(compat)?;
    let reg = nodes.reg()?.next()?;
    Some(MemoryRegion::new(reg.starting_address.cast_mut(), reg.size))
}
