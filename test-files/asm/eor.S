    .text
    .globl _start
_start:
    | Test EOR (Exclusive OR)
    move.l #0xFFFFFFFF, %d0
    move.l #0xAAAAAAAA, %d1
    eor.l %d1, %d0           | d0 = 0xFFFFFFFF ^ 0xAAAAAAAA = 0x55555555
    cmp.l #0x55555555, %d0
    bne fail

    | Test EOR with zero
    move.l #0x12345678, %d2
    move.l #0x00000000, %d3
    eor.l %d3, %d2           | d2 = 0x12345678 ^ 0 = 0x12345678
    cmp.l #0x12345678, %d2
    bne fail

    | Test EORI (EOR immediate)
    move.l #0xFFFFFFFF, %d4
    eori.l #0xAAAAAAAA, %d4  | d4 = 0xFFFFFFFF ^ 0xAAAAAAAA = 0x55555555
    cmp.l #0x55555555, %d4
    bne fail

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0
