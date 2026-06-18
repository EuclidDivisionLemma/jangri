# BUILD_TYPE should be debug or release
BUILD_TYPE=debug

make: build
	cd kernel && cargo run

build: sh fs
	cd kernel && cargo build

clean:
	cargo clean

sh:
ifeq ($(BUILD_TYPE), debug)
	cd ./userspace/src/sh && cargo build --target riscv64imac-unknown-none-elf
else ifeq ($(BUILD_TYPE), release)
	cd ./userspace/src/sh && cargo build --target riscv64imac-unknown-none-elf --release
endif
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/sh ./ramfs.img
	# riscv64-unknown-elf-gcc -nostdlib -T./userspace/userspace.ld ./sh.S -o ./ramfs.img

fs:
