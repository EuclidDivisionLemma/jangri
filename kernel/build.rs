fn main() {
    println!("cargo::rerun-if-changed=src/trampoline.S");
    println!("cargo::rerun-if-changed=./src/init.S");
    cc::Build::new()
        .file("./src/trampoline.S")
        .no_default_flags(true)
        .flag("-march=rv64imac_zicsr")
        .flag("-mabi=lp64")
        .compiler("riscv64-unknown-elf-gcc")
        .compile("trampoline");

    println!("cargo::rustc-link-arg=-Tkernel/main.ld");
}
