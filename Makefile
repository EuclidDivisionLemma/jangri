make: build
	cd kernel && export RUSTFLAGS="-C force-frame-pointers=yes" && cargo run

build: build_process1 build_process2
	cd kernel && cargo build

build_process1: ./kernel/src/process1.S
	mkdir -p ./build
	riscv64-unknown-elf-gcc -c -march=rv64imac_zicsr -mabi=lp64 ./kernel/src/process1.S -o ./build/process1.o
	riscv64-unknown-elf-ld  -entry=process1 ./build/process1.o  -o ./build/process1

build_process2: ./kernel/src/process2.S
	riscv64-unknown-elf-gcc -c -march=rv64imac_zicsr -mabi=lp64 ./kernel/src/process2.S -o ./build/process2.o
	riscv64-unknown-elf-ld  -entry=process2 ./build/process2.o  -o ./build/process2

clean:
	cd kernel && cargo clean

sh:
	riscv64-unknown-elf-gcc -specs=picolibc.specs -nostartfiles -march=rv64imac_zicsr -mabi=lp64 -T./userspace/userspace.ld ./userspace/crt0.S ./userspace/sh.c ./userspace/syscalls.c -o ./userspace/sh.elf
