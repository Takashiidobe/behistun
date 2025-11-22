    .text
    .globl _start

_start:
    /* Test CHK2 and CMP2 instructions (68020+) */

    /* Setup test data with bounds */
    lea     bounds, %a0

    /* Test 1: CMP2 - Value within bounds */
    move.l  #50, %d0            /* D0 = 50 (within bounds 10-100) */
    /* CMP2.L (A0), D0 */
    /* Opcode: 0000 0100 11010 000 = 0x04D0 (CMP2.L with (A0) addressing) */
    /* Extension: 0x0000 (D/A=0, reg=D0, CMP2=0) */
    .word   0x04D0              /* CMP2.L (A0) */
    .word   0x0000              /* D0, CMP2 */

    /* Z should be clear (not at boundary), C should be clear (in bounds) */
    bcs     fail                /* Branch if C set (out of bounds) */

    /* Test 2: CMP2 - Value at lower boundary */
    move.l  #10, %d1            /* D1 = 10 (at lower bound) */
    /* CMP2.L (A0), D1 */
    .word   0x04D0              /* CMP2.L (A0) */
    .word   0x1000              /* D1, CMP2 */

    /* Z should be set (at boundary), C should be clear (in bounds) */
    bcs     fail
    beq     test3               /* Z should be set */
    bra     fail

test3:
    /* Test 3: CMP2 - Value at upper boundary */
    move.l  #100, %d2           /* D2 = 100 (at upper bound) */
    /* CMP2.L (A0), D2 */
    .word   0x04D0              /* CMP2.L (A0) */
    .word   0x2000              /* D2, CMP2 */

    /* Z should be set (at boundary), C should be clear (in bounds) */
    bcs     fail
    beq     test4
    bra     fail

test4:
    /* Test 4: CMP2 - Value below lower bound */
    move.l  #5, %d3             /* D3 = 5 (below lower bound) */
    /* CMP2.L (A0), D3 */
    .word   0x04D0              /* CMP2.L (A0) */
    .word   0x3000              /* D3, CMP2 */

    /* C should be set (out of bounds) */
    bcc     fail                /* Branch if C clear (in bounds) */

test5:
    /* Test 5: CMP2 - Value above upper bound */
    move.l  #150, %d4           /* D4 = 150 (above upper bound) */
    /* CMP2.L (A0), D4 */
    .word   0x04D0              /* CMP2.L (A0) */
    .word   0x4000              /* D4, CMP2 */

    /* C should be set (out of bounds) */
    bcc     fail

test6:
    /* Test 6: CHK2 - Value within bounds (should not trap) */
    move.l  #50, %d5            /* D5 = 50 (within bounds) */
    /* CHK2.L (A0), D5 */
    .word   0x04D0              /* CHK2.L (A0) */
    .word   0x5800              /* D5, CHK2 (bit 11 = 1) */

    /* Should continue execution (no exception) */

test7:
    /* Test 7: CHK2 with address register */
    move.l  #50, %a1            /* A1 = 50 (within bounds) */
    /* CHK2.L (A0), A1*/
    .word   0x04D0              /* CHK2.L (A0) */
    .word   0x9800              /* A1, CHK2 (D/A=1, reg=1, bit11=1) */

    /* Should continue execution */

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
bounds:
    .long   10          /* Lower bound */
    .long   100         /* Upper bound */
