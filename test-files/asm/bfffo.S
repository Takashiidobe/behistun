    .text
    .globl _start
_start:
    | Test BFFFO on data register
    move.l #0x08000000, %d0   | First 1-bit at position 27 (from MSB)
    bfffo %d0{0:32}, %d1      | Expect offset 4 (32 bits - 1 leading nibble)
    cmp.l #4, %d1
    bne fail

    | Success: exit 0
    moveq   #1, %d0
    moveq   #0, %d1
    trap    #0

fail:
    moveq   #1, %d0
    moveq   #1, %d1
    trap    #0
