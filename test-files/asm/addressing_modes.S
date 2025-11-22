    .text
    .globl _start
_start:
    | Test various addressing modes

    | Address register indirect
    lea test_data, %a0
    move.l (%a0), %d0
    cmp.l #0x11111111, %d0
    bne fail

    | Address register indirect with postincrement
    move.l (%a0)+, %d1
    cmp.l #0x11111111, %d1
    bne fail
    move.l (%a0)+, %d2
    cmp.l #0x22222222, %d2
    bne fail

    | Address register indirect with predecrement
    move.l -(%a0), %d3
    cmp.l #0x22222222, %d3
    bne fail

    | Address register indirect with displacement
    lea test_data, %a1
    move.l 4(%a1), %d4
    cmp.l #0x22222222, %d4
    bne fail

    | Address register indirect with index
    move.l #4, %d5
    lea test_data, %a2
    move.l (%a2,%d5.l), %d6
    cmp.l #0x22222222, %d6
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0

    .data
test_data:
    .long 0x11111111
    .long 0x22222222
    .long 0x33333333
