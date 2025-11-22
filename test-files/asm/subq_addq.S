    .text
    .globl _start
_start:
    | Test SUBQ and ADDQ (already tested in moveq_quick.S but more coverage)
    move.l #100, %d0
    subq.l #8, %d0           | 100 - 8 = 92
    cmp.l #92, %d0
    bne fail

    addq.l #8, %d0           | 92 + 8 = 100
    cmp.l #100, %d0
    bne fail

    | Test with address registers
    move.l #1000, %a0
    addq.l #4, %a0
    cmp.l #1004, %a0
    bne fail

    subq.l #4, %a0
    cmp.l #1000, %a0
    bne fail

    | Test word size
    move.w #50, %d1
    subq.w #8, %d1
    cmp.w #42, %d1
    bne fail

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0
