    .text
    .globl _start
_start:
    | Test EXT - extend sign

    | EXT.W - extend byte to word
    move.w #0xFF80, %d0         | -128 as byte
    ext.w %d0
    cmp.w #0xFF80, %d0          | Should still be 0xFF80
    bne fail

    move.w #0x007F, %d1         | +127 as byte
    ext.w %d1
    cmp.w #0x007F, %d1
    bne fail

    | EXT.L - extend word to long
    move.l #0xFFFF8000, %d2     | -32768 as word
    ext.l %d2
    cmp.l #0xFFFF8000, %d2      | Should still be sign-extended
    bne fail

    move.l #0x00007FFF, %d3     | +32767 as word
    ext.l %d3
    cmp.l #0x00007FFF, %d3
    bne fail

    | Test negative byte -> word -> long chain
    move.w #0x0080, %d4         | -128 as byte (0x80)
    ext.w %d4                   | Extend to word: 0xFF80
    ext.l %d4                   | Extend to long: 0xFFFFFF80
    cmp.l #0xFFFFFF80, %d4
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
