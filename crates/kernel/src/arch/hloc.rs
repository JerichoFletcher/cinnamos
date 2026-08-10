cfg_select! {
    target_arch = "riscv64" => {
        use crate::arch::riscv64;
        pub use riscv64::hloc::{HartLocal, hart_local, load_hart_local};
    }
}
