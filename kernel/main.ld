ENTRY(entry)
. = 0x80200000;

SECTIONS
{
    PROVIDE(kernel_start = .);
    .text :
    {
        *(.text);
        *(.text.*);
        . = ALIGN(4096);
    }

    PROVIDE(end_of_kernel_text = .);

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
        . = . + 12582912;
    }
    PROVIDE(stack_top = .);
    PROVIDE(kernel_end = .);
}
