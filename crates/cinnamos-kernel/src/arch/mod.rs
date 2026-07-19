cfg_select! {
    target_arch = "riscv64" => {
        mod riscv64;
        pub(in crate::arch) use riscv64::*;

        pub const KERNEL_LOAD_BASE: usize = 0x8020_0000;
        pub const DIRECT_MAP_BASE: usize = 0xffff_8000_0000_0000;
        pub const KERNEL_MAP_BASE: usize = 0xffff_c000_0000_0000;
        pub const HEAP_MAP_BASE: usize = 0xffff_ff00_0000_0000;
        pub const HEAP_BUMP_SIZE: usize = PageSize::Page4K.size() * 32;

        pub use riscv64::{
            wait_for_interrupt,
            init,
            init_interrupts,
            init_higher_half,
            jump_higher_half,
        };
        pub use riscv64::hloc::{
            load_boot_hart_local,
            hart_local,
        };
        pub use riscv64::paddr::PAddr;
        pub use riscv64::vaddr::VAddr;
        pub use riscv64::context::Context;
        pub use riscv64::sv48::{
            PAGE_SIZE,
            PageSize,
            PageTable,
            PTE,
            PTEFlags,
            map_page,
            unmap_page,
            activate_vmap,
            flush_vmap,
            PageTableAlloc,
            PageTableAllocMap,
            MapError,
            UnmapError,
        };
        pub use riscv64::interrupt::{
            register_irq_handler,
            InterruptError,
        };

        #[cfg(debug_assertions)]
        pub use riscv64::sv48::translate_virt;
    }
}
