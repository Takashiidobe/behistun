    .text
    .globl _start

_start:
    | Allocate 16 bytes on stack
    subq.l  #8, %sp
    subq.l  #8, %sp

    | Test move to stack with pre-decrement addressing
    move.l  %sp, %a0            | Copy sp to a0
    move.l  #0x48656C6C, %d0    | "Hell" in ASCII
    move.l  %d0, (%a0)+         | Store to buffer with post-increment
    move.l  #0x6F2C2057, %d0    | "o, W" in ASCII
    move.l  %d0, (%a0)+         | Store next 4 bytes
    move.l  #0x6F726C64, %d0    | "orld" in ASCII
    move.l  %d0, (%a0)+         | Store next 4 bytes
    move.w  #0x210A, %d0        | "!\n" in ASCII
    move.w  %d0, (%a0)          | Store final 2 bytes

    | Write output
    move.l  #4, %d0             | syscall: write
    move.l  #1, %d1             | fd: stdout
    move.l  %sp, %d2            | buf: stack buffer
    move.l  #14, %d3            | count: 14 bytes
    trap    #0

    | Clean up stack
    addq.l  #8, %sp
    addq.l  #8, %sp

    | Exit
    move.l  #1, %d0             | syscall: exit
    move.l  #0, %d1             | status: 0
    trap    #0
