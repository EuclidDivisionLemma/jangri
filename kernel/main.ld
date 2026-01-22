ENTRY(entry)
. = 0x80200000;


SECTIONS
{
    PROVIDE(kernel_start = .);
    .text :
    {
        *(.text.entry);
        *(.text.*);
        . = ALIGN(4096);
        PROVIDE(trampoline_code_address = .);
        *(trampsec)
        ASSERT(. - trampoline_code_address < 0x1000, "ERROR: TRAMPOLINE LARGER THAN ONE PAGE");
        PROVIDE(end_of_kernel_text = .);

    }

    .rodata :
    {
        *(.rodata);
        *(.rodata.*);
        . = ALIGN(4096);
    }

    .data :
    {
        *(.data);
        *(.data.*);
        . = ALIGN(4096);
    }

    .bss :
    {
        *(.bss);
        *(.bss.*);
        *(COMMON);

        . = ALIGN(4096);
    }

    .stack :
    {
        . = . + 26214400;
    }
    PROVIDE(stack_top = .);
    PROVIDE(kernel_end = .);
}
