    .text
    .globl _start
_start:
    | Test MOVEM - move multiple registers

    | Initialize registers with test values
    move.l #10, %d0
    move.l #20, %d1
    move.l #30, %d2
    move.l #40, %d3

    | Save registers to stack
    movem.l %d0-%d3, -(%sp)

    | Modify registers
    move.l #99, %d0
    move.l #99, %d1
    move.l #99, %d2
    move.l #99, %d3

    | Restore registers from stack
    movem.l (%sp)+, %d0-%d3

    | Verify values were restored
    cmp.l #10, %d0
    bne fail
    cmp.l #20, %d1
    bne fail
    cmp.l #30, %d2
    bne fail
    cmp.l #40, %d3
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
