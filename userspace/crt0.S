.global _start

_start:
    # initialise libc
    jal __libc_init_array

    # push argc and argv into stack
    mv s0, sp
    addi sp, sp, -32
    sd a0, 0(sp)
    sd a1, 8(sp)

    jal main

    li a7, 800
    j __sys_exit
