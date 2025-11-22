    .text
    .globl _start

_start:
    /* Test CAS2 instruction (68020+) */

    /* Setup: Initialize memory locations */
    lea     test_val1, %a0      /* A0 points to first test location */
    lea     test_val2, %a1      /* A1 points to second test location */

    /* Test 1: Successful CAS2 (both values match) */
    move.l  #0x12345678, (%a0)  /* Initialize first location */
    move.l  #0xABCDEF00, (%a1)  /* Initialize second location */

    move.l  #0x12345678, %d0    /* D0 = compare value 1 (matches) */
    move.l  #0xABCDEF00, %d1    /* D1 = compare value 2 (matches) */
    move.l  #0x11111111, %d2    /* D2 = update value 1 */
    move.l  #0x22222222, %d3    /* D3 = update value 2 */

    /* CAS2.L D0:D1, D2:D3, (A0):(A1) */
    /* Opcode: 0x0EFC (CAS2.L) */
    /* Ext 1:  0x8010 (A0, D2, D0) - bit15=1, bits 14-12=000 (A0), bits 5-3=010 (D2), bits 2-0=000 (D0) */
    /* Ext 2:  0x9019 (A1, D3, D1) - bit15=1, bits 14-12=001 (A1), bits 5-3=011 (D3), bits 2-0=001 (D1) */
    .word   0x0EFC
    .word   0x8010
    .word   0x9019

    /* Verify: Both memory locations should now contain update values */
    move.l  (%a0), %d4          /* Load first location */
    cmp.l   #0x11111111, %d4    /* Should be D2's value */
    bne     fail

    move.l  (%a1), %d5          /* Load second location */
    cmp.l   #0x22222222, %d5    /* Should be D3's value */
    bne     fail

    /* Test 2: Failed CAS2 (first value doesn't match) */
    move.l  #0x33333333, (%a0)  /* Initialize first location */
    move.l  #0x44444444, (%a1)  /* Initialize second location */

    move.l  #0xFFFFFFFF, %d0    /* D0 = compare value 1 (won't match) */
    move.l  #0x44444444, %d1    /* D1 = compare value 2 (would match) */
    move.l  #0x55555555, %d2    /* D2 = update value 1 */
    move.l  #0x66666666, %d3    /* D3 = update value 2 */

    /* CAS2.L D0:D1, D2:D3, (A0):(A1) */
    .word   0x0EFC
    .word   0x8010
    .word   0x9019

    /* When CAS2 fails, it loads memory values into Dc registers */
    cmp.l   #0x33333333, %d0    /* D0 should now contain (A0) */
    bne     fail
    cmp.l   #0x44444444, %d1    /* D1 should now contain (A1) */
    bne     fail

    /* Memory should be unchanged */
    move.l  (%a0), %d4
    cmp.l   #0x33333333, %d4
    bne     fail
    move.l  (%a1), %d5
    cmp.l   #0x44444444, %d5
    bne     fail

    /* Success */
    move.l  #1, %d0
    move.l  #0, %d1
    trap    #0

fail:
    move.l  #1, %d0
    move.l  #1, %d1
    trap    #0

    .data
    .align  4
test_val1:
    .long   0
test_val2:
    .long   0
