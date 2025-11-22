    .text
    .globl _start
_start:
    | Test MULU - unsigned multiply (16x16=32)

    | Simple multiply
    move.w #10, %d0
    move.w #20, %d1
    mulu %d1, %d0               | 10 * 20 = 200
    cmp.l #200, %d0
    bne fail

    | Multiply with larger numbers
    move.w #1000, %d2
    move.w #50, %d3
    mulu %d3, %d2               | 1000 * 50 = 50000
    cmp.l #50000, %d2
    bne fail

    | Test MULS - signed multiply (16x16=32)

    | Positive * positive
    move.w #100, %d4
    move.w #10, %d5
    muls %d5, %d4               | 100 * 10 = 1000
    cmp.l #1000, %d4
    bne fail

    | Positive * negative
    move.w #50, %d6
    move.w #-10, %d7
    muls %d7, %d6               | 50 * -10 = -500
    cmp.l #-500, %d6
    bne fail

    | Negative * negative
    move.w #-20, %d0
    move.w #-5, %d1
    muls %d1, %d0               | -20 * -5 = 100
    cmp.l #100, %d0
    bne fail

    | Test zero multiply
    move.w #12345, %d2
    move.w #0, %d3
    mulu %d3, %d2
    cmp.l #0, %d2
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
