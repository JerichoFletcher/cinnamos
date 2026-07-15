use alloc::vec::Vec;
use fdt::{Fdt, node::FdtNode};

use crate::mem::MemoryRegion;

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
