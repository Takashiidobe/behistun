    .text
    .globl _start

_start:
    /* Test RTD instruction (68010+) */
    /* RTD #<displacement> - Return and Deallocate */

    /* Setup: Save initial stack pointer */
    move.l  %a7, %a6            /* Save SP to A6 for later comparison */

    /* Test 1: RTD with positive displacement (deallocate 8 bytes) */
    /* Push two long words (8 bytes) onto stack as "parameters" */
    move.l  #0x12345678, -(%a7) /* Push param 2 */
    move.l  #0xABCDEF00, -(%a7) /* Push param 1 */

    /* Call subroutine that will use RTD #8 to clean up */
    bsr     func_with_rtd_8

    /* After RTD, SP should be back to original value (A6) */
    cmp.l   %a6, %a7
    bne     fail

    /* Test 2: RTD with different displacement (deallocate 12 bytes) */
    move.l  #0x11111111, -(%a7) /* Push param 3 */
    move.l  #0x22222222, -(%a7) /* Push param 2 */
    move.l  #0x33333333, -(%a7) /* Push param 1 */

    bsr     func_with_rtd_12

    /* After RTD, SP should be back to original value */
    cmp.l   %a6, %a7
    bne     fail

    /* Test 3: RTD with zero displacement (like RTS) */
    bsr     func_with_rtd_0

    /* SP should still be at original value */
    cmp.l   %a6, %a7
    bne     fail

    /* Success */
    move.l  #1, %d0
    move.l  #0, %d1
    trap    #0

func_with_rtd_8:
    /* Do some work with parameters if desired */
    /* Return and deallocate 8 bytes */
    /* RTD #8 has opcode 0x4E74 followed by 0x0008 */
    .word   0x4E74
    .word   0x0008

func_with_rtd_12:
    /* Return and deallocate 12 bytes */
    .word   0x4E74
    .word   0x000C

func_with_rtd_0:
    /* Return with no deallocation (equivalent to RTS) */
    .word   0x4E74
    .word   0x0000

fail:
    move.l  #1, %d0
    move.l  #1, %d1
    trap    #0
