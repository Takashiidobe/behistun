    .text
    .globl _start
_start:
    | Test PACK and UNPK instructions (68020)
    | PACK converts unpacked BCD to packed BCD
    | UNPK converts packed BCD to unpacked BCD

    | PACK: Convert 0x0109 (unpacked BCD for 19) to 0x19 (packed BCD)
    move.w #0x0109, %d0
    pack %d0, %d1, #0           | Pack into d1

    | Check result (lower byte should be 0x19)
    and.l #0xFF, %d1
    cmp.l #0x19, %d1
    bne fail

    | UNPK: Convert 0x19 (packed BCD) back to unpacked
    move.w #0x0019, %d2
    unpk %d2, %d3, #0           | Unpack into d3

    | Check result (should be 0x0109)
    and.l #0xFFFF, %d3
    cmp.l #0x0109, %d3
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
