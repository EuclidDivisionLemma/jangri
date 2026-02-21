An educational xv6-style kernel built to explore interrupt handling, context switching and privilege changes.

# Todo
* Process creation using syscalls
* File system
* Testing it with multiple cpus enabled

# Building and Running
* Download `picolibc`, compile and install using:

```
  meson setup build --cross-file cross-riscv64.txt --prefix=/opt/cross/ -Dposix-io=true -Dmultilib=false   -Dnewlib-multithread=false -Dnewlib-retargetable-locking=false -Dc_args="-march=rv64imac_zicsr -mabi=lp64"   -Dc_link_args="-march=rv64imac_zicsr -mabi=lp64"  -Dposix-console=true -Dthread-local-storage=false
  ```
  
* Install `qemu`, and Rust toolchain

* Run `make` in the root directory
