    .text
    .globl _start
_start:
    | Test ASL/ASR - arithmetic shift left/right
    | ASL is same as LSL (left shift fills with 0)
    | ASR preserves sign bit (right shift)

    | ASL - arithmetic shift left
    move.l #0x00000001, %d0
    asl.l #1, %d0
    cmp.l #0x00000002, %d0
    bne fail

    move.l #0x00000005, %d1
    asl.l #2, %d1               | 5 << 2 = 20
    cmp.l #20, %d1
    bne fail

    | ASR - arithmetic shift right (sign-extending)
    move.l #0x80000000, %d2     | Negative number
    asr.l #1, %d2
    cmp.l #0xC0000000, %d2      | Sign bit preserved
    bne fail

    move.l #0x00000008, %d3     | Positive number
    asr.l #2, %d3               | 8 >> 2 = 2
    cmp.l #2, %d3
    bne fail

    | Test LSL/LSR - logical shift left/right
    | LSL is same as ASL
    | LSR always shifts in zeros (no sign extension)

    | LSL - logical shift left
    move.l #0x00000003, %d4
    lsl.l #3, %d4               | 3 << 3 = 24
    cmp.l #24, %d4
    bne fail

    | LSR - logical shift right (zero-fill)
    move.l #0x80000000, %d5     | MSB set
    lsr.l #1, %d5
    cmp.l #0x40000000, %d5      | Zero shifted in
    bne fail

    move.l #0xF0000000, %d6
    lsr.l #4, %d6
    cmp.l #0x0F000000, %d6
    bne fail

    | Test byte shifts
    move.b #0x80, %d7
    asr.b #1, %d7
    and.l #0xFF, %d7
    cmp.l #0xC0, %d7            | Sign extended
    bne fail

    | Success
    move.l #1, %d0
    move.l #0, %d1
    trap #0

fail:
    move.l #1, %d0
    move.l #1, %d1
    trap #0
