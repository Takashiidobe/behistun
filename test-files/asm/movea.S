    .text
    .globl _start
_start:
    | Test MOVEA (Move to Address register)
    move.l #0x12345678, %d0
    movea.l %d0, %a0         | Copy d0 to a0
    cmp.l %d0, %a0
    bne fail

    | Test MOVEA with immediate
    movea.l #0xABCDEF00, %a1
    cmp.l #0xABCDEF00, %a1
    bne fail

    | Test MOVEA.W (word - sign extended to long)
    movea.w #0x8000, %a2     | Negative word
    cmp.l #0xFFFF8000, %a2   | Should be sign-extended
    bne fail

    movea.w #0x1234, %a3     | Positive word
    cmp.l #0x00001234, %a3   | Should be zero-extended high bits
    bne fail

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0
