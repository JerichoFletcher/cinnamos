#![feature(ptr_metadata)]
#![feature(layout_for_ptr)]

#![no_std]
#![no_main]

mod macros;
mod asm;
mod panic;

pub mod arch;
pub mod mem;
pub mod device;
pub mod dtb;
