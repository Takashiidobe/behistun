.global _start
_start:
    moveq   #5, %d0
    moveq   #7, %d1
    add.l   %d1, %d0       /* 12 */
    sub.l   #3, %d0        /* 9 */

    move.l  %d0, %d1       /* status = 9 */
    moveq   #1, %d0        /* exit */
    trap    #0
