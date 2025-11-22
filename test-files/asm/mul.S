.global _start
_start:
    moveq   #6, %d0
    moveq   #7, %d1
    mulu.l  %d1, %d0    /* 42 */

    move.l  %d0, %d1    /* status 42 */
    moveq   #1, %d0     /* exit */
    trap    #0
