#![feature(associated_type_defaults)]
#![no_std]
#![no_main]

// extern crate alloc;

pub mod arch;
pub mod cpu;
pub mod time;
pub mod page;
pub mod mem;
pub mod sched;
pub mod console;
pub mod bits;

mod panic;
mod trap;
