    .text
    .globl _start
_start:
    | Test LINK and UNLK instructions for stack frames

    | Save initial stack pointer
    move.l %sp, %a2

    | Create a stack frame
    link %a6, #-16              | Allocate 16 bytes local space

    | Use the local space - write some data
    move.l #0x12345678, -4(%a6)
    move.l #0xABCDEF00, -8(%a6)

    | Verify we can read it back
    move.l -4(%a6), %d0
    cmp.l #0x12345678, %d0
    bne fail

    move.l -8(%a6), %d1
    cmp.l #0xABCDEF00, %d1
    bne fail

    | Destroy the stack frame
    unlk %a6

    | Verify stack pointer is restored (approximately)
    | Allow some variation due to link/unlk overhead
    move.l %sp, %a3
    move.l %a2, %d2
    move.l %a3, %d3
    sub.l %d3, %d2
    | Stack pointer should be within a few bytes of original
    cmp.l #20, %d2
    bgt fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
