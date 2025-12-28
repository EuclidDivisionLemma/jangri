.global _start

_start:
    # initialise libc
    jal __libc_init_array

    # push argc and argv into stack
    addi sp, sp, -16
    sd a0, 0(sp)
    sd a1, 8(sp)
    addi sp, sp, 16

    jal main
    j loop

loop:
    j loop
