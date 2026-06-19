fn main() {
    println!("cargo::rustc-link-arg=-T./userspace/userspace.ld");
}
