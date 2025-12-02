.section .text
.global entry
entry:
    la sp, stack_top
    j main
