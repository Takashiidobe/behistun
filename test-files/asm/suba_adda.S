    .text
    .globl _start
_start:
    | Test SUBA (Subtract from Address register)
    move.l #1000, %a0
    move.l #100, %d0
    suba.l %d0, %a0          | a0 = 1000 - 100 = 900
    cmp.l #900, %a0
    bne fail

    | Test ADDA (Add to Address register)
    move.l #500, %d1
    adda.l %d1, %a0          | a0 = 900 + 500 = 1400
    cmp.l #1400, %a0
    bne fail

    | Test with immediate
    suba.l #400, %a0         | a0 = 1400 - 400 = 1000
    cmp.l #1000, %a0
    bne fail

    adda.l #234, %a0         | a0 = 1000 + 234 = 1234
    cmp.l #1234, %a0
    bne fail

    | Test word size (sign extended)
    move.l #5000, %a1
    suba.w #100, %a1         | Subtract word
    cmp.l #4900, %a1
    bne fail

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0
