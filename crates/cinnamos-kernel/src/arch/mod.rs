cfg_select! {
    target_arch = "riscv64" => {
        mod riscv64;
        pub(in crate::arch) use riscv64::*;

        pub const DIRECT_MAP_BASE: usize = 0xffff_8000_0000_0000;
        pub const KERNEL_MAP_BASE: usize = 0xffff_c000_0000_0000;

        pub use riscv64::{
            init,
            init_higher_half,
            jump_to_higher_half,
        };
        pub use riscv64::paddr::PAddr;
        pub use riscv64::vaddr::VAddr;
        pub use riscv64::sv48::{
            PAGE_SIZE,
            PageSize,
            PageTable,
            PTE,
            PTEFlags,
            map_page,
            unmap_page,
            activate_vmap,
            PageTableAlloc,
            PageTableAllocMap,
            MapError,
            UnmapError,
        };

        #[cfg(debug_assertions)]
        pub use riscv64::sv48::translate_virt;
    }
}
