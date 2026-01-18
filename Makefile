make: build
	cd kernel && export RUSTFLAGS="-C force-frame-pointers=yes" && cargo run

build: init
	cd kernel && cargo build

clean:
	cd kernel && cargo clean

init:
	riscv64-unknown-elf-gcc -specs=picolibc.specs -nostartfiles -march=rv64imac_zicsr -mabi=lp64 -T./userspace/userspace.ld -fomit-frame-pointer ./userspace/crt0.S ./userspace/init.c ./userspace/syscalls.c -o ./userspace/init.elf
