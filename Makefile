make: build
	cd kernel && export RUSTFLAGS="-C force-frame-pointers=yes" && cargo run

build: process1 process2 init
	cd kernel && cargo build

clean:
	cd kernel && cargo clean

init:
	riscv64-unknown-elf-gcc -specs=picolibc.specs -nostartfiles -march=rv64imac_zicsr -mabi=lp64 -T./userspace/userspace.ld -fomit-frame-pointer ./userspace/crt0.S ./userspace/init.c ./userspace/syscalls.c -o ./userspace/init.elf

process1:
	riscv64-unknown-elf-as  -march=rv64imac_zicsr -mabi=lp64 ./userspace/process1.S -o ./userspace/process1.o
	riscv64-unknown-elf-ld ./userspace/process1.o -o ./userspace/process1.elf -T./userspace/userspace.ld

process2:
	riscv64-unknown-elf-as  -march=rv64imac_zicsr -mabi=lp64 ./userspace/process2.S -o ./userspace/process2.elf
	riscv64-unknown-elf-ld ./userspace/process2.o -o ./userspace/process2.elf -T./userspace/userspace.ld
