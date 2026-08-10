cfg_select! {
    target_arch = "riscv64" => {
        use crate::arch::riscv64;
        pub use riscv64::sv48::{
            MapError, PAGE_SIZE, PAGE_TABLE_DEPTH, PTE, PTEFlags, PageLevel, PageTable, UnmapError,
            flush_address_space, flush_address_space_at, get_max_asid, map_page,
            switch_address_space, translate_virt, unmap_page,
        };
    }
}
