    .text
    .globl _start
_start:
    | Test MOVEQ - move quick (sign-extended immediate)

    | Positive values
    moveq #0, %d0
    cmp.l #0, %d0
    bne fail

    moveq #127, %d1
    cmp.l #127, %d1
    bne fail

    | Negative values (sign-extended)
    moveq #-1, %d2
    cmp.l #-1, %d2
    bne fail

    moveq #-128, %d3
    cmp.l #-128, %d3
    bne fail

    | Test ADDQ - add quick
    move.l #100, %d4
    addq.l #5, %d4
    cmp.l #105, %d4
    bne fail

    addq.l #8, %d4
    cmp.l #113, %d4
    bne fail

    | Test ADDQ with #1
    move.l #999, %d5
    addq.l #1, %d5
    cmp.l #1000, %d5
    bne fail

    | Test SUBQ - subtract quick
    move.l #100, %d6
    subq.l #3, %d6
    cmp.l #97, %d6
    bne fail

    subq.l #7, %d6
    cmp.l #90, %d6
    bne fail

    | Test with address register
    move.l #1000, %a0
    addq.l #4, %a0
    cmp.l #1004, %a0
    bne fail

    subq.l #4, %a0
    cmp.l #1000, %a0
    bne fail

    | Test byte and word sizes
    move.w #100, %d7
    addq.w #5, %d7
    cmp.w #105, %d7
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
