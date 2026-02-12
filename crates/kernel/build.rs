fn main() {
    let target = std::env::var("TARGET").unwrap();
    let mut build = cc::Build::new();

    if target.contains("riscv32") {
        build.file("src/asm/arch_riscv32.s")
            .flag("--target=riscv32-unknown-elf");
    }
    
    build.compiler("clang")
        .no_default_flags(true)
        .compile("arch_entry");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/boot/");
    println!("cargo:rerun-if-changed=src/asm/");
}
