    .text
    .globl _start

_start:
    /* Test BKPT instruction (68010+) */
    /* BKPT #<vector> where vector is 0-7 */

    /* BKPT has opcode: 0100 1000 0100 1nnn */
    /* where nnn is the vector number (0-7) */

    /* Test BKPT #0 */
    .word   0x4848              /* BKPT #0 */

    /* Test BKPT #3 */
    .word   0x484B              /* BKPT #3 */

    /* Test BKPT #7 */
    .word   0x484F              /* BKPT #7 */

    /* If we reach here, BKPT was handled correctly (as no-op for testing) */
    move.l  #1, %d0
    move.l  #0, %d1             /* Success */
    trap    #0
