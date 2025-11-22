.global _start
_start:
    moveq   #3, %d0
    moveq   #2, %d1
    cmp.l   %d1, %d0      /* sets Z/N/V/C */
    ble.s   fail

    moveq   #1, %d0       /* exit */
    moveq   #11, %d1      /* status if branch taken */
    trap    #0

fail:
    moveq   #1, %d0
    moveq   #1, %d1
    trap    #0
