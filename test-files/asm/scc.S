    .text
    .globl _start
_start:
    | Test Scc - Set according to condition
    move.l #0, %d0

    | Test SEQ - set if equal
    moveq #5, %d1
    cmp.l #5, %d1
    seq %d0
    and.l #0xFF, %d0
    cmp.l #0xFF, %d0           | Should be 0xFF (true)
    bne fail

    | Test SNE - set if not equal
    moveq #5, %d2
    cmp.l #10, %d2
    sne %d0
    and.l #0xFF, %d0
    cmp.l #0xFF, %d0           | Should be 0xFF (true)
    bne fail

    | Test SGT - set if greater than
    moveq #10, %d3
    cmp.l #5, %d3
    sgt %d0
    and.l #0xFF, %d0
    cmp.l #0xFF, %d0           | Should be 0xFF (true)
    bne fail

    | Test SLT - set if less than
    moveq #3, %d4
    cmp.l #8, %d4
    slt %d0
    and.l #0xFF, %d0
    cmp.l #0xFF, %d0           | Should be 0xFF (true)
    bne fail

    | Test false condition
    moveq #5, %d5
    cmp.l #5, %d5
    sne %d0
    and.l #0xFF, %d0
    cmp.l #0, %d0              | Should be 0 (false)
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
