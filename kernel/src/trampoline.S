.section trampsec
.global handle_traps_from_user_mode
.global return_to_user_mode
.align 4

handle_traps_from_user_mode:
    csrw sscratch, a0
    li a0, 0xFFFFFFFFFFFFE000

    sd ra, 0(a0)
    sd sp, 8(a0)
    sd gp, 16(a0)
    sd tp, 24(a0)

    sd t0, 32(a0)
    sd t1, 40(a0)
    sd t2, 48(a0)

    sd s0, 56(a0)
    sd s1, 64(a0)

    sd a1, 80(a0)
    sd a2, 88(a0)
    sd a3, 96(a0)
    sd a4, 104(a0)
    sd a5, 112(a0)
    sd a6, 120(a0)
    sd a7, 128(a0)

    sd s2, 136(a0)
    sd s3, 144(a0)
    sd s4, 152(a0)
    sd s5, 160(a0)
    sd s6, 168(a0)
    sd s7, 176(a0)
    sd s8, 184(a0)
    sd s9, 192(a0)
    sd s10, 200(a0)
    sd s11, 208(a0)

    sd t3, 216(a0)
    sd t4, 224(a0)
    sd t5, 232(a0)
    sd t6, 240(a0)

    csrr t0, sscratch
    sd t0, 72(a0)


    ld sp, 264(a0) # Set sp to point to the process's kernel stack
    li t0, 4096
    li t1, 6
    mul t0, t0, t1
    add sp, sp, t0

    # load the kernel page table address
    ld t0, 272(a0)

    ld t1, 280(a0) # load the address of usertrap()

    sfence.vma

    csrw satp, t0

    sfence.vma

    jalr t1


return_to_user_mode:
    sfence.vma

    # load user page table
    ld a1, 288(a0)

    csrw satp, a1

    sfence.vma

    li a0, 0xFFFFFFFFFFFFE000

    ld ra, 0(a0)
    ld sp, 8(a0)
    ld gp, 16(a0)
    ld tp, 24(a0)

    ld t0, 32(a0)
    ld t1, 40(a0)
    ld t2, 48(a0)

    ld s0, 56(a0)
    ld s1, 64(a0)

    ld a1, 80(a0)
    ld a2, 88(a0)
    ld a3, 96(a0)
    ld a4, 104(a0)
    ld a5, 112(a0)
    ld a6, 120(a0)
    ld a7, 128(a0)

    ld s2, 136(a0)
    ld s3, 144(a0)
    ld s4, 152(a0)
    ld s5, 160(a0)
    ld s6, 168(a0)
    ld s7, 176(a0)
    ld s8, 184(a0)
    ld s9, 192(a0)
    ld s10, 200(a0)
    ld s11, 208(a0)

    ld t3, 216(a0)
    ld t4, 224(a0)
    ld t5, 232(a0)
    ld t6, 240(a0)

    ld a0, 72(a0)

    sret
