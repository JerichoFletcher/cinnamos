#![feature(
    ptr_metadata,
    layout_for_ptr,
    coroutines,
    iter_from_coroutine,
    atomic_ptr_null
)]
#![no_std]
#![no_main]

extern crate alloc;

mod asm;
mod macros;
mod panic;

pub mod arch;
pub mod device;
pub mod hloc;
pub mod mem;
// pub mod sched;
pub mod devicetree;
pub mod rel;
pub mod sym;
