    .text
    .globl _start
_start:
    | Test BFCHG: invert top nibble of d0
    move.l #0x12345670, %d0
    bfchg %d0{28:4}           | Expect 0xE2345670
    cmp.l #0xE2345670, %d0
    bne fail

    | Test BFCLR: clear middle byte of d1
    move.l #0xFFFF0000, %d1
    bfclr %d1{4:8}            | Clear bits 27-20
    cmp.l #0xF00F0000, %d1
    bne fail

    | Test BFSET: set all bits of d2
    move.l #0, %d2
    bfset %d2{0:32}
    cmp.l #-1, %d2
    bne fail

    | Success: exit 0
    moveq   #1, %d0
    moveq   #0, %d1
    trap    #0

fail:
    moveq   #1, %d0
    moveq   #1, %d1
    trap    #0
