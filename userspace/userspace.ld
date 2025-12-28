ENTRY(_start)

SECTIONS
{

    . = 0x1000;

    .text :
    {
        *(.text);
        *(.text.*);
    }

    . = ALIGN(4096);

    .rodata :
    {
        *(.rodata);
        *(.rodata.*);
    }

    . = ALIGN(4096);

    .data :
    {
        *(.data);
        *(.data.*);
    }

    . = ALIGN(4096);

    PROVIDE(__bss_start = .);
    .bss :
    {
        *(.bss);
        *(.bss.*);
    }
    PROVIDE(__bss_end = .);
    PROVIDE(__bss_size = SIZEOF(.bss));

    . = ALIGN(4096);
}
