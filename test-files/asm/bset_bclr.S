    .text
    .globl _start
_start:
    | Test BSET and BCLR instructions
    move.l #0, %d0

    | Set bit 0
    bset #0, %d0
    cmp.l #1, %d0
    bne fail

    | Set bit 3
    bset #3, %d0
    cmp.l #9, %d0              | Binary: 1001
    bne fail

    | Set bit 7
    bset #7, %d0
    cmp.l #137, %d0            | Binary: 10001001
    bne fail

    | Clear bit 0
    bclr #0, %d0
    cmp.l #136, %d0            | Binary: 10001000
    bne fail

    | Clear bit 7
    bclr #7, %d0
    cmp.l #8, %d0              | Binary: 1000
    bne fail

    | Test BTST
    move.l #0xFF, %d1
    btst #7, %d1               | Test bit 7 (should be set)
    beq fail                   | If not set, fail

    btst #8, %d1               | Test bit 8 (should be clear)
    bne fail                   | If set, fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
