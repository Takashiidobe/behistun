    .text
    .globl _start
_start:
    | Test NEG and NOT instructions

    | NEG - negate (two's complement)
    move.l #100, %d0
    neg.l %d0
    cmp.l #-100, %d0
    bne fail

    move.l #-50, %d1
    neg.l %d1
    cmp.l #50, %d1
    bne fail

    | NOT - logical complement (one's complement)
    move.l #0x0000FFFF, %d2
    not.l %d2
    cmp.l #0xFFFF0000, %d2
    bne fail

    move.l #0xAAAAAAAA, %d3
    not.l %d3
    cmp.l #0x55555555, %d3
    bne fail

    | NEG with zero
    move.l #0, %d4
    neg.l %d4
    cmp.l #0, %d4
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
