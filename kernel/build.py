import subprocess


def free_standing_compile(input: str, output: str):
    subprocess.run(
        f"riscv64-unknown-elf-gcc -c -nostdlib -nostartfiles -ffreestanding {input} -o {output}",
        shell=True,
    )


def free_standing_link(input: str, output: str, flags: str, entry: str):
    subprocess.run(
        f"riscv64-unknown-elf-ld -entry={entry} {input} -o {output} {flags}", shell=True
    )


if __name__ == "__main__":
    free_standing_compile("./src/init.S", "./target/init.o")
    free_standing_link("./target/init.o", "./target/init.bin", "", "init")
