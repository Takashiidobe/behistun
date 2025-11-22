    .text
    .globl _start
_start:
    | Test ROL/ROR - rotate left/right (no extend)

    | ROL - rotate left
    move.l #0x12345678, %d0
    rol.l #4, %d0               | Rotate left by 4 bits
    cmp.l #0x23456781, %d0
    bne fail

    | ROR - rotate right
    move.l #0x12345678, %d1
    ror.l #4, %d1               | Rotate right by 4 bits
    cmp.l #0x81234567, %d1
    bne fail

    | Test with #1 rotation
    move.l #0x80000000, %d2
    rol.l #1, %d2               | MSB should move to LSB
    cmp.l #0x00000001, %d2
    bne fail

    move.l #0x00000001, %d3
    ror.l #1, %d3               | LSB should move to MSB
    cmp.l #0x80000000, %d3
    bne fail

    | Test ROXL/ROXR - rotate with extend
    move #0, %ccr               | Clear X flag

    move.l #0x80000000, %d4
    roxl.l #1, %d4              | Rotate left through X
    | X flag gets MSB (1), MSB gets old X (0)
    cmp.l #0x00000000, %d4
    bne fail

    | X flag should now be 1, rotate it back in
    move.l #0, %d5
    roxl.l #1, %d5              | LSB gets X flag (1)
    cmp.l #0x00000001, %d5
    bne fail

    | Test byte rotation
    move.b #0x81, %d6
    rol.b #1, %d6
    and.l #0xFF, %d6
    cmp.l #0x03, %d6            | 0x81 -> 0x03
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
