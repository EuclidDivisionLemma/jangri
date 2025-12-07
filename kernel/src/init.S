.section .rodata
msg:
    .string "Jangri v0.0.1"

.section .text.init
.global init

init:
    li a7, 0x98846
    j cont

cont:
    j cont
