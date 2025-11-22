    .text
    .globl _start
_start:
    | Test BFEXTS - bit field extract signed (68020)
    | In m68k: bit 31=MSB, bit 0=LSB
    | offset 0 = start from bit 31, offset 24 = start from bit 7

    | Test 1: Extract low byte with MSB=0 (should not sign extend)
    move.l #0x00000056, %d0
    bfexts %d0{24:8}, %d1       | Extract bits 7-0: 0x56 (MSB=0)
    cmp.l #0x00000056, %d1      | Should be positive
    bne fail

    | Test 2: Extract low byte with MSB=1 (should sign extend)
    move.l #0x000000FF, %d0
    bfexts %d0{24:8}, %d2       | Extract bits 7-0: 0xFF (MSB=1)
    cmp.l #-1, %d2              | Should be -1 (0xFFFFFFFF)
    bne fail

    | Test 3: Extract from middle (bits 15-8)
    move.l #0x00005678, %d0
    bfexts %d0{16:8}, %d3       | Extract bits 15-8: 0x56 (MSB=0)
    cmp.l #0x00000056, %d3
    bne fail

    | Test 4: Extract negative value from middle
    move.l #0x0000FF78, %d0
    bfexts %d0{16:8}, %d4       | Extract bits 15-8: 0xFF (MSB=1)
    cmp.l #-1, %d4              | Should be -1
    bne fail

    | Test 5: Extract 4-bit field without sign extension
    move.l #0x00000007, %d0
    bfexts %d0{28:4}, %d5       | Extract bits 3-0: 0x7 (MSB=0)
    cmp.l #0x00000007, %d5
    bne fail

    | Test 6: Extract 4-bit field with sign extension
    move.l #0x0000000F, %d0
    bfexts %d0{28:4}, %d6       | Extract bits 3-0: 0xF (MSB=1)
    cmp.l #-1, %d6              | Should be -1 (sign extended)
    bne fail

    | Test 7: Extract negative value 0x80 (bit 7 set)
    move.l #0x00000080, %d0
    bfexts %d0{24:8}, %d7       | Extract bits 7-0: 0x80 (MSB=1)
    move.l #0xFFFFFF80, %d1
    cmp.l %d1, %d7              | Should be -128 (0xFFFFFF80)
    bne fail

    | Test 8: Extract 16-bit signed value
    move.l #0x00008000, %d0
    bfexts %d0{16:16}, %d1      | Extract bits 15-0: 0x8000 (MSB=1)
    move.l #0xFFFF8000, %d2
    cmp.l %d2, %d1              | Should be sign-extended
    bne fail

    | Test 9: Extract positive 16-bit value
    move.l #0x00007FFF, %d0
    bfexts %d0{16:16}, %d1      | Extract bits 15-0: 0x7FFF (MSB=0)
    cmp.l #0x00007FFF, %d1
    bne fail

    | Success - return 42 as exit code
    move.l #1, %d0
    move.l #42, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
