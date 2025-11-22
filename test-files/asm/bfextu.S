    .text
    .globl _start
_start:
    | Test BFEXTU - bit field extract unsigned (68020)
    | Extract bits from a 32-bit value
    move.l #0x12345678, %d0

    | Extract bits 8-15 (offset 8, width 8)
    bfextu %d0{8:8}, %d1        | Should extract 0x56

    cmp.l #0x56, %d1
    bne fail

    | Extract bits 16-23 (offset 16, width 8)
    bfextu %d0{16:8}, %d2       | Should extract 0x34

    cmp.l #0x34, %d2
    bne fail

    | Extract bits 0-7 (offset 0, width 8)
    bfextu %d0{0:8}, %d3        | Should extract 0x78

    cmp.l #0x78, %d3
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
