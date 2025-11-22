    .text
    .globl _start
_start:
    | Test CAS - Compare and Swap (68020)
    | CAS compares Dc with memory location, if equal, stores Du to memory

    | Set up test data in memory
    move.l #100, -(%sp)         | Push 100 onto stack

    | Test case 1: Compare matches, should swap
    move.l #100, %d0            | Compare value
    move.l #200, %d1            | Update value
    cas.l %d0, %d1, (%sp)       | If *sp == d0, *sp = d1

    | Check that memory was updated to 200
    move.l (%sp), %d2
    cmp.l #200, %d2
    bne fail

    | Test case 2: Compare doesn't match, should not swap
    move.l #999, %d3            | Compare value (wrong)
    move.l #300, %d4            | Update value
    cas.l %d3, %d4, (%sp)       | Should not update since 200 != 999

    | Check that memory still contains 200
    move.l (%sp), %d5
    cmp.l #200, %d5
    bne fail

    | Clean up stack
    addq.l #4, %sp

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    addq.l #4, %sp
    move.l #1, %d0
    move.l #1, %d1
    trap #0
