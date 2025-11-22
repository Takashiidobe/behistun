.global _start
_start:
    move.l  #0xf0, %d0
    and.l   #0x3c, %d0    /* 0x30 */
    or.l    #0x05, %d0    /* 0x35 -> 53 */

    move.l  %d0, %d1       /* status 53 */
    moveq   #1, %d0       /* exit */
    trap    #0
