    .text
    .globl _start
_start:
    | Test 32-bit multiply (68020)
    move.l #1000, %d0
    move.l #2000, %d1
    muls.l %d1, %d0             | d0 = 1000 * 2000 = 2000000

    cmp.l #2000000, %d0
    bne fail

    | Test 32-bit divide (68020)
    move.l #10000, %d2
    move.l #100, %d3
    divs.l %d3, %d2             | d2 = 10000 / 100 = 100

    cmp.l #100, %d2
    bne fail

    | Test with negative numbers
    move.l #-500, %d4
    move.l #10, %d5
    muls.l %d5, %d4             | d4 = -500 * 10 = -5000

    cmp.l #-5000, %d4
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
