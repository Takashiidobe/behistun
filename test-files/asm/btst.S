    .text
    .globl _start
_start:
    | Test BTST (Bit Test)
    move.l #0x80000000, %d0
    btst #31, %d0            | Test bit 31 (MSB)
    beq fail                 | Should be set (Z=0)

    btst #30, %d0            | Test bit 30
    bne fail                 | Should be clear (Z=1)

    | Test BTST with register bit number
    move.l #0x00000100, %d1  | Bit 8 is set
    moveq #8, %d2
    btst %d2, %d1            | Test bit 8
    beq fail

    moveq #7, %d2
    btst %d2, %d1            | Test bit 7
    bne fail

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0
