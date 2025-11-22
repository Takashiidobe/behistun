.global _start
_start:
    move.l  #0x7fffffff, %d0
    add.l   #1, %d0          /* wraps to 0x80000000 */
    move.l  %d0, %d1         /* status = 0x80000000 -> exit code truncated */
    moveq   #1, %d0          /* exit */
    trap    #0
