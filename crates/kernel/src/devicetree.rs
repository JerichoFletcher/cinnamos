use alloc::vec::Vec;
use fdt::{Fdt, node::FdtNode};

use crate::{
    arch::{InterruptSource, PAddr},
    mem::{MemoryRegion, RegionSubtract, SizedMemoryRegion},
};

/// Finds the first node with a `compatible` property that intersects with the given string slice,
/// along with its memory region, or [`None`] if no such node is found.
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

/// Finds all nodes that has an `interrupts` property and has the given interrupt parent.
pub fn all_with_interrupts<'b, 'a: 'b>(
    fdt: &'b Fdt<'a>,
    interrupt_parent: &FdtNode<'b, 'a>,
) -> impl Iterator<Item = (FdtNode<'b, 'a>, Vec<InterruptSource>)> {
    core::iter::from_coroutine(
        #[coroutine]
        || {
            for n in fdt.all_nodes() {
                if let Some(intp) = n.interrupt_parent()
                    && intp.name == interrupt_parent.name
                    && let Some(ints) = n
                        .interrupts()
                        .map(|ints| ints.filter_map(InterruptSource::new))
                {
                    let ints = ints.collect::<Vec<_>>();
                    yield (n, ints)
                }
            }
        },
    )
}

/// Collects all usable and reserved region slices defined in the devicetree.
/// The caller can provide a slice of additional reserved regions for the purpose of usable region slicing.
///
/// All returned usable regions are guaranteed to be disjoint (i.e. no intersections between any two regions).
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
                .flatten(),
        );
    }
    rsv_regs.sort();

    let mut usable_regs: Vec<SizedMemoryRegion> = Vec::with_capacity(rsv_regs.len() + 1);
    for r in fdt
        .memory()
        .regions()
        .filter_map(|r| SizedMemoryRegion::new(PAddr::from_ptr(r.starting_address), r.size))
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
    for rsv in rsv {
        if reg.intersects(rsv) {
            match reg.subtract(rsv) {
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
