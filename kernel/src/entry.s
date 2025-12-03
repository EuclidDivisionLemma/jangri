.section .text.entry
.global entry
entry:
    la sp, stack_top
    j main
