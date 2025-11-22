    .text
    .globl _start
_start:
    | Simple addition test
    move.l #10, %d0
    move.l #20, %d1
    add.l %d1, %d0

    | d0 should now be 30
    | Exit with exit code 30
    move.l #1, %d0
    move.l #30, %d1
    trap #0
