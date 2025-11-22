    .text
    .globl _start
_start:
    | Test ADDX - add with extend (carry)

    | Clear X flag first
    move #0, %ccr

    | Add two numbers with potential carry
    move.l #0xFFFFFFFF, %d0
    move.l #1, %d1
    addx.l %d1, %d0             | Should be 0 with carry set

    | d0 should be 0
    cmp.l #0, %d0
    bne fail

    | X flag should be set, so next addx will add 1
    move.l #100, %d2
    move.l #200, %d3
    addx.l %d3, %d2             | 100 + 200 + X(1) = 301

    cmp.l #301, %d2
    bne fail

    | Test SUBX - subtract with extend
    move #0, %ccr               | Clear X flag

    move.l #500, %d4
    move.l #200, %d5
    subx.l %d5, %d4             | 500 - 200 - 0 = 300

    cmp.l #300, %d4
    bne fail

    | Test SUBX with borrow
    move.l #0, %d6
    move.l #1, %d7
    subx.l %d7, %d6             | 0 - 1 = -1 (0xFFFFFFFF)

    cmp.l #0xFFFFFFFF, %d6
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
