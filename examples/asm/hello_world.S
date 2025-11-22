.text
.globl _start

_start:
    /* write(1, message, len) */
    moveq   #1, %d1             /* arg1: fd = stdout */
    lea     message, %a0
    move.l  %a0, %d2            /* arg2: buf */
    move.l  #message_len, %d3   /* arg3: count */
    moveq   #4, %d0             /* syscall: write */
    trap    #0

    /* exit(0) */
    moveq   #1, %d0             /* syscall: exit */
    moveq   #0, %d1             /* status = 0 */
    trap    #0

.data
message:
    .ascii "Hello, world!\n"
    .set message_len, . - message
