# BUILD_TYPE should be debug or release
BUILD_TYPE=release

make: build
	cd kernel && cargo run

build: maths sh
	cd kernel && cargo build

clean:
	cargo clean
	rm ./maths.bin
	rm ./sh.bin

sh: maths
ifeq ($(BUILD_TYPE), debug)
	cd ./userspace/src/sh && cargo build --target riscv64imac-unknown-none-elf
else ifeq ($(BUILD_TYPE), release)
	cd ./userspace/src/sh && cargo build --target riscv64imac-unknown-none-elf --release
endif
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/sh ./sh.bin


maths:
	cp ./userspace/src/sh/main.rs ./userspace/src/sh/main1.rs
	sed -i "11d" ./userspace/src/sh/main.rs
	sed -i "24d" ./userspace/src/sh/main.rs
ifeq ($(BUILD_TYPE), debug)
	cd ./userspace/src/maths && cargo build --target riscv64imac-unknown-none-elf
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/maths ./maths.bin
else ifeq ($(BUILD_TYPE), release)
	cd ./userspace/src/maths && cargo build --target riscv64imac-unknown-none-elf --release
	cp ./target/riscv64imac-unknown-none-elf/$(BUILD_TYPE)/maths ./maths.bin
endif
	mv ./userspace/src/sh/main1.rs ./userspace/src/sh/main.rs
