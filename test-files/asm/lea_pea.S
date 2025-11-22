    .text
    .globl _start
_start:
    | Test LEA - load effective address
    lea test_data, %a0
    move.l (%a0), %d0
    cmp.l #0xDEADBEEF, %d0
    bne fail

    | LEA with displacement
    lea 4(%a0), %a1
    move.l (%a1), %d1
    cmp.l #0xCAFEBABE, %d1
    bne fail

    | LEA with index
    move.l #8, %d2
    lea (%a0,%d2.l), %a2
    move.l (%a2), %d3
    cmp.l #0x12345678, %d3
    bne fail

    | Test PEA - push effective address
    lea test_data, %a3
    pea (%a3)                   | Push address onto stack
    move.l (%sp)+, %a4          | Pop it back

    | Verify it's the same address
    cmp.l %a3, %a4
    bne fail

    | Verify we can access data through popped address
    move.l (%a4), %d4
    cmp.l #0xDEADBEEF, %d4
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0

    .data
test_data:
    .long 0xDEADBEEF
    .long 0xCAFEBABE
    .long 0x12345678
