fn main() {
    println!("cargo:rerun-if-changed=src/arch/riscv/entry.s");
    println!("cargo:rerun-if-changed=src/arch/riscv/tvec.s");
    cc::Build::new()
        .file("src/arch/riscv/entry.s")
        .file("src/arch/riscv/tvec.s")
        .compiler("clang")
        .no_default_flags(true)
        .flag("--target=riscv32-unknown-elf")
        .compile("arch_entry");
}
