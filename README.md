A simple xv6-style kernel.

# Note
The `master` branch has only a shell and supports only two shell commands `about` and `echo`. There are no other userspace programs. The `spawn` branch contains one userspace program other than the shell that just prints a greeting. To check the working of the new spawn syscall, which is available only on the `spawn` branch, switch to the `spawn` branch.

# Planned 
## High priority
* Complete the implementation of `Spawn` syscall.
* Add more userspace programs and a **proper** RamFS

## Low priority

* Support for SMP
* Simple but persistent disk-backed (QEMU drive image) file system

# Repo Organisation

`allocator` - Contains the physical allocator used to allocate physical pages. This is not used for the kernel heap allocator. The kernel heap is a statically allocated buffer.

`hal` - Contains the interfaces (traits) necessary for architecture-specific code to implement.

`arch` - Contains architecture-specific implementations of the traits in the `hal` crate.

`kernel` - The crate houses the code for interrupt handling, sycall handling, context switching, and scheduling.

`sync` - Contains architecture-independent code for `Mutex` (xv6-style spinlocks), `RwLock`.

`userspace` - Contains sample userspace programs.

`janglib` - This crate is included by both the kernel and userspace programs. For the userspace programs, this crate provides the necessary syscall wrappers and library calls to do common tasks like printing to stdout, reading from stdin, allocating memory etc.

# Running

* Install `qemu` and the Rust Risc-V toolchain.
* Run `make` in the root directory of this project.
