.global _start
_start:
    moveq   #0, %d0
    moveq   #5, %d1
    moveq   #7, %d2

    /* push d1, d2 */
    move.l  %d1, -(%sp)
    move.l  %d2, -(%sp)

    /* pop into d0 and add */
    move.l  (%sp)+, %d0    /* d0 = 7 */
    add.l   (%sp)+, %d0    /* d0 += 5 => 12 */

    move.l  %d0, %d1       /* status 12 */
    moveq   #1, %d0
    trap    #0
