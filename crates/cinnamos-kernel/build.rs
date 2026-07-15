fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:link-arg=-T{}/src/ld/riscv64.lds", root);
    println!("cargo:rerun-if-changed={}/src/ld/riscv64.lds", root);
    println!("cargo:rerun-if-changed={}/src/ld/virt.lds", root);
}
