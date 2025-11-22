    .text
    .globl _start
_start:
    | Test CMP instruction variations
    | CMP is already tested, but let's test CMPA and CMPI

    | Test CMPA (Compare Address)
    move.l #1000, %a0
    move.l #1000, %a1
    cmpa.l %a1, %a0
    bne fail

    move.l #2000, %a2
    cmpa.l %a2, %a0
    beq fail                 | Should not be equal

    | Test CMPI (Compare Immediate)
    move.l #12345, %d0
    cmpi.l #12345, %d0
    bne fail

    cmpi.l #99999, %d0
    beq fail

    | Test CMPM is tested separately, so just verify word size
    move.w #100, %d1
    cmpi.w #100, %d1
    bne fail

    | Success
    move.l #1, %d0           | SYS_exit
    moveq #0, %d1            | exit code
    trap #0

fail:
    move.l #1, %d0           | SYS_exit
    moveq #1, %d1            | exit code
    trap #0
