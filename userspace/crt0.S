.global _start

_start:
    # initialise libc
    jal __libc_init_array
    jal main

    li a7, 800
    j exit
