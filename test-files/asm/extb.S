    .text
    .globl _start
_start:
    | Test EXTB.L - 68020 instruction to extend byte to long
    move.l #0xFFFFFF80, %d0    | -128 as a byte in lower 8 bits
    extb.l %d0                  | Extend byte to long (should be 0xFFFFFF80)

    | Check if result is still 0xFFFFFF80 (sign extended)
    cmp.l #0xFFFFFF80, %d0
    bne fail

    | Test with positive byte
    move.l #0xFFFFFF7F, %d1    | +127 as a byte
    extb.l %d1                  | Should become 0x0000007F
    cmp.l #0x0000007F, %d1
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
