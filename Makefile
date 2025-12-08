make: build
	cd kernel && cargo run

build: build_process1 build_process2
	mkdir -p ./build
	cd kernel && cargo build

build_process1: ./kernel/src/process1.S
	riscv64-unknown-elf-gcc -c -march=rv64imac_zicsr -mabi=lp64 ./kernel/src/process1.S -o ./build/process1.o
	riscv64-unknown-elf-ld  -entry=process1 ./build/process1.o  -o ./build/process1

build_process2: ./kernel/src/process2.S
	riscv64-unknown-elf-gcc -c -march=rv64imac_zicsr -mabi=lp64 ./kernel/src/process2.S -o ./build/process2.o
	riscv64-unknown-elf-ld  -entry=process2 ./build/process2.o  -o ./build/process2
