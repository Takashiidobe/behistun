    .text
    .globl _start

_start:
    | Test move to memory locations
    lea     buffer(%pc), %a0    | Load address of buffer
    move.l  #0x48656C6C, %d0    | "Hell" in ASCII
    move.l  %d0, (%a0)+         | Store to buffer with post-increment
    move.l  #0x6F2C2057, %d0    | "o, W" in ASCII
    move.l  %d0, (%a0)+         | Store next 4 bytes
    move.l  #0x6F726C64, %d0    | "orld" in ASCII
    move.l  %d0, (%a0)+         | Store next 4 bytes
    move.w  #0x210A, %d0        | "!\n" in ASCII
    move.w  %d0, (%a0)          | Store final 2 bytes

    | Write output - test pre-decrement
    move.l  #4, %d0             | syscall: write
    move.l  #1, %d1             | fd: stdout
    lea     buffer(%pc), %a0
    move.l  %a0, %d2            | buf: buffer
    move.l  #14, %d3            | count: 14 bytes
    trap    #0

    | Exit
    move.l  #1, %d0             | syscall: exit
    move.l  #0, %d1             | status: 0
    trap    #0

buffer:
    .space  16                  | 16 bytes of space (in .text so PC-relative works)
