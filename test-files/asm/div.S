.global _start
_start:
    move.l  #1000, %d0
    move.l  #7, %d1
    divu.w  %d1, %d0       /* quotient in upper word, remainder in lower */
    /* After divu.w: d0 = quotient:remainder (16 bits each) */
    move.w  %d0, %d1       /* remainder */
    moveq   #1, %d0        /* exit */
    trap    #0
