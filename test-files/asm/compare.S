    .text
    .globl _start
_start:
    | Test comparison and conditional branches
    move.l #10, %d0
    move.l #20, %d1

    cmp.l %d0, %d1
    bgt greater         | d1 > d0, should branch

    | Should not reach here
    move.l #99, %d1
    bra exit

greater:
    | Should reach here
    move.l #42, %d1

exit:
    | Exit with d1 as status (should be 42)
    move.l #1, %d0
    trap #0
