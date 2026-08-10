pub mod addr;
pub mod hloc;
pub mod interrupt;
pub mod virt;

cfg_select! {
    target_arch = "riscv64" => {
        mod riscv64;
        pub(in crate::arch) use riscv64::*;

        /// The physical address the kernel is loaded at.
        pub const KERNEL_LOAD_BASE: usize = 0x0000_0000_8020_0000;
        /// The base address of the direct map space.
        pub const DIRECT_MAP_BASE: usize = 0xffff_8000_0000_0000;
        /// The base address of the kernel space.
        pub const KERNEL_MAP_BASE: usize = 0xffff_c000_0000_0000;
        /// The base address of the kernel virtual memory allocation space.
        pub const VMALLOC_MAP_BASE: usize = 0xffff_f000_0000_0000;
        /// The end address of the kernel virtual memory allocation space.
        pub const VMALLOC_MAP_END: usize = 0xffff_ff00_0000_0000;
        /// The base address of the kernel heap space.
        pub const HEAP_MAP_BASE: usize = 0xffff_ff00_0000_0000;

        pub use riscv64::console::get_fallback_console;
        pub use riscv64::context::Context;
        pub use riscv64::hart::{HartStartError, start_hart};
        pub use riscv64::task::Task;
        pub use riscv64::trap::TrapFrame;
        pub use riscv64::{
            ElfDyn, get_dyn, init, init_higher_half, jump_higher_half, wait_for_interrupt,
        };
    }
}
