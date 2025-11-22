    .text
    .globl _start
_start:
    | Test TST - test operand and set condition codes

    | TST with zero
    move.l #0, %d0
    tst.l %d0
    bne fail                    | Should set Z flag

    | TST with positive
    move.l #100, %d1
    tst.l %d1
    beq fail                    | Should clear Z flag
    bmi fail                    | Should clear N flag

    | TST with negative
    move.l #-50, %d2
    tst.l %d2
    beq fail                    | Should clear Z flag
    bpl fail                    | Should set N flag

    | Test CLR - clear operand
    move.l #0x12345678, %d3
    clr.l %d3
    cmp.l #0, %d3
    bne fail

    | CLR should also set Z flag
    move.l #999, %d4
    clr.l %d4
    bne fail                    | Z flag should be set

    | Test CLR.W - only clears lower word
    move.l #0x12345678, %d5
    clr.w %d5
    cmp.l #0x12340000, %d5
    bne fail

    | Test CLR.B - only clears lower byte
    move.l #0x12345678, %d6
    clr.b %d6
    cmp.l #0x12345600, %d6
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
