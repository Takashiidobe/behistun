    .text
    .globl _start

_start:
    | Test post-increment read
    lea     message(%pc), %a0   | Load address of message
    move.l  (%a0)+, %d0         | Read first 4 bytes with post-increment
    move.l  (%a0)+, %d1         | Read next 4 bytes with post-increment

    | Write output
    move.l  #4, %d0             | syscall: write
    move.l  #1, %d1             | fd: stdout
    lea     message(%pc), %a0
    move.l  %a0, %d2            | buf: message
    move.l  #14, %d3            | count: 14 bytes
    trap    #0

    | Exit
    move.l  #1, %d0             | syscall: exit
    move.l  #0, %d1             | status: 0
    trap    #0

message:
    .ascii  "Hello, World!\n"
