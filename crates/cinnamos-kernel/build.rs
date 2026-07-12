fn main() {
    println!("cargo:rerun-if-changed=src/ld/riscv64.lds");
    println!("cargo:rerun-if-changed=src/ld/virt.lds");
}
