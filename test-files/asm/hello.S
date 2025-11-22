.global _start
_start:
    moveq   #4, %d0          /* write */
    moveq   #1, %d1          /* stdout */
    lea     msg, %a0
    moveq   #13, %d2
    trap    #0

    moveq   #1, %d0          /* exit */
    moveq   #0, %d1
    trap    #0

msg:
    .ascii "hello from asm\n"
