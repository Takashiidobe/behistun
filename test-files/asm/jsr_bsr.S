    .text
    .globl _start
_start:
    | Test JSR/RTS - jump to subroutine and return
    move.l #10, %d0
    move.l #20, %d1

    jsr add_func

    | Result should be in d0
    cmp.l #30, %d0
    bne fail

    | Test BSR - branch to subroutine
    move.l #100, %d0
    move.l #50, %d1

    bsr sub_func

    | Result should be in d0
    cmp.l #50, %d0
    bne fail

    | Test nested subroutine calls
    move.l #5, %d0
    jsr double_func
    cmp.l #10, %d0
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

add_func:
    add.l %d1, %d0
    rts

sub_func:
    sub.l %d1, %d0
    rts

double_func:
    add.l %d0, %d0              | Double the value
    rts

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
