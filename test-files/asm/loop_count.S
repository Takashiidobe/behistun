    .text
    .globl _start
_start:
    | Count from 1 to 10 and sum
    move.l #0, %d0      | sum = 0
    move.l #1, %d1      | i = 1

loop:
    add.l %d1, %d0      | sum += i
    addq.l #1, %d1      | i++
    cmp.l #11, %d1      | if i < 11
    blt loop            | continue loop

    | d0 should be 55 (1+2+3+...+10)
    | Exit with sum as status
    move.l #1, %d0
    move.l #55, %d1
    trap #0
