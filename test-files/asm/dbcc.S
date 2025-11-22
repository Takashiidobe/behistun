    .text
    .globl _start
_start:
    | Test DBcc - decrement and branch conditionally

    | Test DBRA (DBF) - loop counter
    move.l #0, %d0              | Sum
    move.w #9, %d1              | Counter (0-9 = 10 iterations)

loop1:
    add.l #1, %d0               | Increment sum
    dbra %d1, loop1             | Decrement d1 and loop if not -1

    | d0 should be 10 (counted 10 times)
    cmp.l #10, %d0
    bne fail

    | d1 should be -1 (0xFFFF in lower word)
    cmp.w #-1, %d1
    bne fail

    | Test DBcc with condition that becomes true
    move.l #0, %d2
    move.w #5, %d3

loop2:
    addq.l #1, %d2
    cmp.l #3, %d2               | Set condition when d2 == 3
    dbeq %d3, loop2             | Loop while not equal and d3 >= 0

    | Should have exited when d2 == 3
    cmp.l #3, %d2
    bne fail

    | Test DBNE - decrement and branch while not equal
    move.l #100, %d4
    move.w #5, %d5

loop3:
    subq.l #1, %d4
    cmp.l #96, %d4
    dbne %d5, loop3             | Loop while d4 != 96

    | Should have stopped when d4 == 96
    cmp.l #96, %d4
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
