use alloc::vec::Vec;
use fdt::{Fdt, node::FdtNode};

use crate::{
    arch::PAddr,
    mem::{MemoryRegion, RegionSubtract, SizedMemoryRegion},
};

pub fn find_compatible<'b, 'a: 'b>(
    fdt: &'a Fdt,
    compat: &'a [&str],
) -> Option<(FdtNode<'b, 'a>, MemoryRegion)> {
    let node = fdt.find_compatible(compat)?;
    let reg = node.reg()?.next()?;
    Some((
        node,
        MemoryRegion::new(reg.starting_address.cast_mut(), reg.size),
    ))
}

pub fn all_with_interrupts<'b, 'a: 'b>(
    fdt: &'b Fdt<'a>,
    interrupt_parent: &FdtNode<'b, 'a>,
) -> impl Iterator<Item = (FdtNode<'b, 'a>, Vec<usize>)> {
    core::iter::from_coroutine(
        #[coroutine]
        || {
            for n in fdt.all_nodes() {
                if let Some(intp) = n.interrupt_parent()
                    && intp.name == interrupt_parent.name
                    && let Some(ints) = n.interrupts()
                {
                    let ints: Vec<usize> = ints.collect();
                    yield (n, ints)
                }
            }
        },
    )
}

pub fn get_region_slices<const N: usize>(
    fdt: &Fdt,
    add_rsv: [SizedMemoryRegion; N],
) -> (Vec<SizedMemoryRegion>, Vec<SizedMemoryRegion>) {
    let mut rsv_regs: Vec<SizedMemoryRegion> = fdt
        .memory_reservations()
        // Safety: r.size() is never zero
        .map(|r| unsafe {
            SizedMemoryRegion::new_unchecked(PAddr::from_ptr(r.address()), r.size())
        })
        .chain(add_rsv)
        .into_iter()
        .collect();
    if let Some(rsv) = fdt.find_node("/reserved-memory") {
        rsv_regs.extend(
            rsv.children()
                .map(|n| n.reg())
                .filter_map(|r| {
                    r.map(|rs| {
                        rs.map(|r| {
                            SizedMemoryRegion::new(PAddr::from_ptr(r.starting_address), r.size)
                        })
                    })
                })
                .flatten()
                .filter_map(|r| r),
        );
    }
    rsv_regs.sort();

    let mut usable_regs: Vec<SizedMemoryRegion> = Vec::with_capacity(rsv_regs.len() + 1);
    for r in fdt
        .memory()
        .regions()
        .map(|r| SizedMemoryRegion::new(PAddr::from_ptr(r.starting_address), r.size))
        .filter_map(|r| r)
    {
        slice_usable_region(r, &mut rsv_regs, &mut usable_regs);
    }
    (usable_regs, rsv_regs)
}

fn slice_usable_region(
    reg: SizedMemoryRegion,
    rsv: &mut [SizedMemoryRegion],
    out: &mut Vec<SizedMemoryRegion>,
) {
    rsv.sort_unstable();

    let mut reg = reg;
    for i in 0..rsv.len() {
        if reg.overlaps(&rsv[i]) {
            match reg.subtract(&rsv[i]) {
                RegionSubtract::None => return,
                RegionSubtract::Left(reg_l) => {
                    out.push(reg_l);
                    return;
                }
                RegionSubtract::Right(reg_r) => reg = reg_r,
                RegionSubtract::Both(reg_l, reg_r) => {
                    out.push(reg_l);
                    reg = reg_r;
                }
                RegionSubtract::Nonoverlapping => (),
            }
        }
    }
    out.push(reg);
}
