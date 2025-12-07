.section .rodata
msg:
    .string "Sample\n"
.align 4

.section .text
.global init

.align 4
init:
    li a7, 0
    la a0, msg
    li a1, 7
    ecall
    j cont

.align 4
cont:
    j cont
