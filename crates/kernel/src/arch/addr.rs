cfg_select! {
    target_arch = "riscv64" => {
        use crate::arch::riscv64;
        pub use riscv64::paddr::PAddr;
        pub use riscv64::vaddr::VAddr;
    }
}
