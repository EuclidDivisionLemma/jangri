fn main() {
    if std::env::var("TARGET").unwrap() == "riscv64imac-unknown-none-elf" {
        println!("cargo::rerun-if-changed=../arch/riscv/src/trampoline.S");
        cc::Build::new()
            .file("../arch/riscv/src/trampoline.S")
            .no_default_flags(true)
            .flag("-march=rv64imac_zicsr")
            .flag("-mabi=lp64")
            .compiler("riscv64-unknown-elf-gcc")
            .compile("trampoline");
    } else {
        panic!("{}: Unsupported Target", std::env::var("TARGET").unwrap())
    };
    println!("cargo::rerun-if-changed=./src/init.S");

    println!("cargo::rustc-link-arg=-Tkernel/main.ld");
}
