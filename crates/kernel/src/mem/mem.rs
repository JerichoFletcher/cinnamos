use crate::arch::mem::{PT, PTEFlags, PAGE_SIZE, PAGE_SIZE_ORD};
use crate::mem::kernel;
use crate::println;

pub fn id_map_range(root: &mut PT, start: usize, end: usize, flags: PTEFlags) {
    let mut addr = start & !(PAGE_SIZE - 1);
    let num_pages = (crate::bits::align_next(end, PAGE_SIZE_ORD) - addr) / PAGE_SIZE;

    for _ in 0..num_pages {
        crate::arch::mem::map(root, addr, addr, flags);
        addr += PAGE_SIZE;
    }
}

pub fn init() {
    kernel::init();

    unsafe {
        unsafe extern "C" {
            static TEXT_START: usize;
            static TEXT_END: usize;
            static RODATA_START: usize;
            static RODATA_END: usize;
            static DATA_START: usize;
            static DATA_END: usize;
            static BSS_START: usize;
            static BSS_END: usize;
            static KSTACK_START: usize;
            static KSTACK_END: usize;
        }

        let root = kernel::page_table().as_mut();
        println!("\nKernel Identity Mapping");
        println!("====================================");
        println!("TEXT      : 0x{:x} -> 0x{:x}", TEXT_START, TEXT_END);
        println!("RODATA    : 0x{:x} -> 0x{:x}", RODATA_START, RODATA_END);
        println!("DATA      : 0x{:x} -> 0x{:x}", DATA_START, DATA_END);
        println!("BSS       : 0x{:x} -> 0x{:x}", BSS_START, BSS_END);
        println!("KSTACK    : 0x{:x} -> 0x{:x}", KSTACK_START, KSTACK_END);
        println!("====================================");

        id_map_range(root, TEXT_START, TEXT_END, PTEFlags::RX);
        id_map_range(root, RODATA_START, RODATA_END, PTEFlags::RX);
        id_map_range(root, DATA_START, DATA_END, PTEFlags::RW);
        id_map_range(root, BSS_START, BSS_END, PTEFlags::RW);
        id_map_range(root, KSTACK_START, KSTACK_END, PTEFlags::RW);

        crate::arch::mem::enable_paging(root);
    }
}
