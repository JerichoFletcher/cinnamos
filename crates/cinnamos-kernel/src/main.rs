#![no_std]
#![no_main]

use fdt::Fdt;
use cinnamos_kernel::*;
use cinnamos_kernel::mem::*;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub extern "C" fn kinit_pt(_id: usize, dtb_ptr: *const u8) -> usize {
    arch::boot::init_pt(dtb_ptr)
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain(_id: usize, dtb_ptr: *const u8) -> ! {
    arch::init();

    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid FDT") };
    init_memory(&fdt);
    cpu::idle();
}

fn init_memory(fdt: &Fdt) {
    if let Some(mem_first) = fdt.memory().regions().next() && let Some(mem_size) = mem_first.size {
        mem::page::init(PhysAddr::from_ptr(mem_first.starting_address), mem_size);
    } else {
        panic!("Cannot initialize memory: no available memory region");
    }
}
