    .text
    .globl _start
_start:
    | Test CMPM (Compare Memory to Memory)
    | Set up two memory areas
    lea data1, %a0
    lea data2, %a1

    | Compare equal values
    cmpm.l (%a0)+, (%a1)+    | Compare and post-increment
    bne fail                 | Should be equal

    | Check that pointers advanced
    lea data1, %a2
    add.l #4, %a2
    cmp.l %a2, %a0
    bne fail

    | Compare different values
    lea data3, %a3
    lea data4, %a4
    cmpm.l (%a3)+, (%a4)+
    beq fail                 | Should not be equal

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0

    .data
data1:
    .long 0x12345678
data2:
    .long 0x12345678
data3:
    .long 0x11111111
data4:
    .long 0x22222222
