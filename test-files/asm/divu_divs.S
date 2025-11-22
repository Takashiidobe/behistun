    .text
    .globl _start
_start:
    | Test DIVU - unsigned divide (32/16 = 16 remainder 16)

    | Simple divide
    move.l #200, %d0
    move.w #10, %d1
    divu %d1, %d0               | 200 / 10 = 20 remainder 0
    | Lower word has quotient, upper word has remainder
    move.l %d0, %d2
    and.l #0xFFFF, %d2          | Extract quotient
    cmp.l #20, %d2
    bne fail

    | Divide with remainder
    move.l #105, %d3
    move.w #10, %d4
    divu %d4, %d3               | 105 / 10 = 10 remainder 5
    move.l %d3, %d5
    and.l #0xFFFF, %d5          | Extract quotient
    cmp.l #10, %d5
    bne fail
    swap %d3
    and.l #0xFFFF, %d3          | Extract remainder
    cmp.l #5, %d3
    bne fail

    | Test DIVS - signed divide

    | Positive / positive
    move.l #1000, %d6
    move.w #25, %d7
    divs %d7, %d6               | 1000 / 25 = 40 remainder 0
    move.l %d6, %d0
    and.l #0xFFFF, %d0
    cmp.l #40, %d0
    bne fail

    | Negative / positive
    move.l #-100, %d1
    move.w #10, %d2
    divs %d2, %d1               | -100 / 10 = -10
    move.l %d1, %d3
    ext.l %d3                   | Sign extend quotient
    cmp.l #-10, %d3
    bne fail

    | Positive / negative
    move.l #100, %d4
    move.w #-10, %d5
    divs %d5, %d4               | 100 / -10 = -10
    move.l %d4, %d6
    ext.l %d6
    cmp.l #-10, %d6
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
