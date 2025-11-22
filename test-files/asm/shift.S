.global _start
_start:
    moveq   #1, %d0
    lsl.l   #4, %d0    /* 16 */
    lsr.l   #2, %d0    /* 4 */

    move.l  %d0, %d1    /* status 4 */
    moveq   #1, %d0    /* exit */
    trap    #0
