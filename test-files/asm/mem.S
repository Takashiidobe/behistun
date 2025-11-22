.data
value1: .long 0x11111111
value2: .long 0x22222222
.text
.global _start
_start:
    move.l  value1, %d0
    move.l  value2, %d1
    add.l   %d1, %d0           /* 0x33333333 */
    move.l  %d0, value1        /* store back */
    clr.l   %d0
    move.l  value1, %d0        /* reload */
    move.l  %d0, %d1           /* status 0x33333333 -> truncated */
    moveq   #1, %d0
    trap    #0
