cfg_select! {
    target_arch = "riscv64" => {
        mod riscv64;
        pub(in crate::arch) use riscv64::*;

        pub const KERNEL_LOAD_BASE: usize   = 0x0000_0000_8020_0000;
        pub const DIRECT_MAP_BASE: usize    = 0xffff_8000_0000_0000;
        pub const KERNEL_MAP_BASE: usize    = 0xffff_c000_0000_0000;
        pub const VMALLOC_MAP_BASE: usize   = 0xffff_f000_0000_0000;
        pub const VMALLOC_MAP_END: usize    = 0xffff_ff00_0000_0000;
        pub const HEAP_MAP_BASE: usize      = 0xffff_ff00_0000_0000;

        pub const SWITCH_FRAME_SIZE: usize = 13 * size_of::<usize>();

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
            PAGE_TABLE_DEPTH,
            PageSize,
            PageTable,
            PTE,
            PTEFlags,
            map_page,
            unmap_page,
            get_max_asid,
            switch_address_space,
            flush_address_space,
            PageTableAlloc,
            PageTableAllocMap,
            MapError,
            UnmapError,
        };
        pub use riscv64::trap::{
            create_task_init_stack,
            TrapFrame,
        };
        pub use riscv64::interrupt::{
            register_irq_handler,
            InterruptError,
        };

        #[cfg(debug_assertions)]
        pub use riscv64::sv48::translate_virt;
    }
}
