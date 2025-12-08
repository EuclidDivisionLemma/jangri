import subprocess
from os import mkdir


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
    try:
        mkdir("./target")
    except Exception:
        pass
    free_standing_compile("./src/process1.S", "./target/process1.o")
    free_standing_link("./target/process1.o", "./target/process1.bin", "", "process1")

    free_standing_compile("./src/process2.S", "./target/process2.o")
    free_standing_link("./target/process2.o", "./target/process2.bin", "", "process2")
    subprocess.run("cargo run", shell=True)
