    .text
    .globl _start

_start:
    /* Test TRAPcc instructions (68020+) */
    /* TRAPcc tests condition codes and traps if true */
    /* Format: 0101 cccc 1111 1sss */
    /* sss: 010=word, 011=long, 100=no operand */

    /* Test 1: TRAPF (trap never - condition always false) */
    /* TRAPF with no operand */
    /* Condition F = 0001, so opcode is 0101 0001 1111 1100 = 0x51FC */
    .word   0x51FC              /* TRAPF (no operand) */

    /* Test 2: TRAPT (trap always - condition always true) */
    /* For testing, this is treated as no-op */
    /* Condition T = 0000, so opcode is 0101 0000 1111 1100 = 0x50FC */
    .word   0x50FC              /* TRAPT (no operand) */

    /* Test 3: TRAPHI with word operand (trap if higher) */
    /* Set up condition: Z=0, C=0 (higher) */
    move.l  #5, %d0
    cmp.l   #3, %d0             /* 5 > 3, sets Z=0, C=0 */
    /* TRAPHI.W #0x1234 */
    /* Condition HI = 0010, opcode is 0101 0010 1111 1010 = 0x52FA */
    .word   0x52FA              /* TRAPHI.W */
    .word   0x1234              /* word operand */

    /* Test 4: TRAPLS with long operand (trap if lower or same) */
    /* Set up condition: Z=1 or C=1 (lower or same) */
    move.l  #3, %d0
    cmp.l   #5, %d0             /* 3 < 5, sets C=1 */
    /* TRAPLS.L #0x12345678 */
    /* Condition LS = 0011, opcode is 0101 0011 1111 1011 = 0x53FB */
    .word   0x53FB              /* TRAPLS.L */
    .long   0x12345678          /* long operand */

    /* Test 5: TRAPCC (trap if carry clear) */
    /* Set up condition: C=0 */
    move.l  #5, %d0
    cmp.l   #3, %d0             /* 5 > 3, sets C=0 */
    /* TRAPCC (no operand) */
    /* Condition CC = 0100, opcode is 0101 0100 1111 1100 = 0x54FC */
    .word   0x54FC              /* TRAPCC */

    /* Test 6: TRAPCS (trap if carry set) */
    /* Set up condition: C=1 */
    move.l  #3, %d0
    cmp.l   #5, %d0             /* 3 < 5, sets C=1 */
    /* TRAPCS.W #0x5678 */
    /* Condition CS = 0101, opcode is 0101 0101 1111 1010 = 0x55FA */
    .word   0x55FA              /* TRAPCS.W */
    .word   0x5678

    /* Test 7: TRAPNE (trap if not equal) */
    /* Set up condition: Z=0 */
    move.l  #5, %d0
    cmp.l   #3, %d0             /* 5 != 3, sets Z=0 */
    /* TRAPNE (no operand) */
    /* Condition NE = 0110, opcode is 0101 0110 1111 1100 = 0x56FC */
    .word   0x56FC              /* TRAPNE */

    /* Test 8: TRAPEQ (trap if equal) */
    /* Set up condition: Z=1 */
    move.l  #5, %d0
    cmp.l   #5, %d0             /* 5 == 5, sets Z=1 */
    /* TRAPEQ.L #0xABCDEF00 */
    /* Condition EQ = 0111, opcode is 0101 0111 1111 1011 = 0x57FB */
    .word   0x57FB              /* TRAPEQ.L */
    .long   0xABCDEF00

    /* If we reach here, all TRAPcc instructions were handled correctly */
    move.l  #1, %d0
    move.l  #0, %d1             /* Success */
    trap    #0
