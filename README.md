An educational xv6-style kernel built to explore interrupt handling, context switching and privilege changes.

The kernel is written such that extending it to other architectures requires only minimal changes in the kernel crate. For allocating pages, the kernel uses a simple but lazy, buddy-style allocator.

# Todo

* Process creation using syscalls
* File system
* Testing it with multiple CPUs enabled

# Repo Organisation

`allocator` - Contains the buddy-style allocator used to allocate physical pages. This is not used for the kernel heap allocator. The kernel heap is a statically allocated buffer.

`hal` - Contains the interfaces (traits) necessary for architecture-specific code to implement.

`arch` - Contains architecture-specific implementations of the traits in the `hal` crate.

`kernel` - The crate houses the code for interrupt handling, system calls, context switching, and scheduling.

`sync` - Contains architecture-independent code for `Mutex` (xv6-style spinlocks), `RwLock`.

`userspace` - Contains two sample userspace programs to illustrate the working of context switching, trap handling, the working of the stack and the heap. The directory also contains the code for the C Runtime and C syscall wrappers.

# Building and Running

* Download `picolibc`, compile using:

```
meson setup build --cross-file cross-riscv64.txt --prefix=/opt/cross/ -Dposix-io=true -Dmultilib=false   -Dnewlib-multithread=false -Dnewlib-retargetable-locking=false -Dc_args="-march=rv64imac_zicsr -mabi=lp64"  -Dc_link_args="-march=rv64imac_zicsr -mabi=lp64"  -Dposix-console=true -Dthread-local-storage=false
```


* Install `qemu` and the Rust toolchain.
* Run `make` in the root directory.
