# BUILD_TYPE should be debug or release
BUILD_TYPE=release

make: build
	cd kernel && cargo run

build: greet sh
	cd kernel && cargo build

clean:
	cargo clean
	rm ./greet.bin
	rm ./sh.bin

sh: greet
ifeq ($(BUILD_TYPE), debug)
	cd ./userspace/src/sh && cargo build --target riscv64imac-unknown-none-elf
else ifeq ($(BUILD_TYPE), release)
	cd ./userspace/src/sh && cargo build --target riscv64imac-unknown-none-elf --release
endif
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/sh ./sh.bin


greet:
	cp ./userspace/src/sh/main.rs ./userspace/src/sh/main1.rs
	sed -i "11d" ./userspace/src/sh/main.rs
	sed -i "24d" ./userspace/src/sh/main.rs
ifeq ($(BUILD_TYPE), debug)
	cd ./userspace/src/greet && cargo build --target riscv64imac-unknown-none-elf
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/greet ./greet.bin
else ifeq ($(BUILD_TYPE), release)
	cd ./userspace/src/greet && cargo build --target riscv64imac-unknown-none-elf --release
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/greet ./greet.bin
endif
	mv ./userspace/src/sh/main1.rs ./userspace/src/sh/main.rs
