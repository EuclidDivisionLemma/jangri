A simple xv6-style kernel.

There are two userspace programs `sh` and `greet`. The `sh` program simply runs an infinite loop waiting for a prompt. The valid prompts currently are `about`, `echo` and the name of the command `greet`. The first two are shell commands (i.e no process is spawned), while `greet` is a seperate executable program which does nothing but print a greeting message.

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
