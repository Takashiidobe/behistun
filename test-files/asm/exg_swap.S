    .text
    .globl _start
_start:
    | Test EXG - exchange registers
    move.l #0x11111111, %d0
    move.l #0x22222222, %d1

    exg %d0, %d1

    cmp.l #0x22222222, %d0
    bne fail
    cmp.l #0x11111111, %d1
    bne fail

    | Test EXG with address registers
    move.l #0xAAAAAAAA, %a0
    move.l #0xBBBBBBBB, %a1

    exg %a0, %a1

    cmp.l #0xBBBBBBBB, %a0
    bne fail
    cmp.l #0xAAAAAAAA, %a1
    bne fail

    | Test SWAP - swap words in register
    move.l #0x12345678, %d2
    swap %d2
    cmp.l #0x56781234, %d2
    bne fail

    | Test SWAP again
    swap %d2
    cmp.l #0x12345678, %d2
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
