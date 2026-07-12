#![feature(
    ptr_metadata,
    layout_for_ptr,
    coroutines,
    iter_from_coroutine,
    atomic_ptr_null,
)]

#![no_std]
#![no_main]

extern crate alloc;

mod macros;
mod asm;
mod panic;

pub mod arch;
pub mod mem;
pub mod hloc;
pub mod device;
// pub mod sched;
pub mod devicetree;
pub mod rel;
pub mod sym;
