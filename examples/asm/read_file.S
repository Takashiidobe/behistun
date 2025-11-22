.text
.globl _start

_start:
    /* open(path, O_RDONLY, 0) */
    lea filename, %a0           /* a0 = &filename */
    move.l %a0, %d1             /* arg1: pathname */
    moveq   #0, %d2             /* arg2: flags = O_RDONLY */
    moveq   #0, %d3             /* arg3: mode (unused for O_RDONLY) */
    moveq   #5, %d0             /* syscall: open */
    trap    #0
    tst.l   %d0
    bmi     exit_fail           /* open failed */
    move.l  %d0, %d7            /* save fd in d7 */

read_loop:
    /* read(fd, buf, count) */
    move.l  %d7, %d1            /* arg1: fd */
    lea     buffer, %a0
    move.l  %a0, %d2            /* arg2: buf */
    move.l  #BUFFER_SIZE, %d3   /* arg3: count */
    moveq   #3, %d0             /* syscall: read */
    trap    #0
    tst.l   %d0
    ble     exit_success        /* <=0: EOF or error */

    /* write(1, buf, nread) */
    moveq   #1, %d1             /* arg1: fd = stdout */
    move.l  %a0, %d2            /* arg2: buf */
    move.l  %d0, %d3            /* arg3: count */
    moveq   #4, %d0             /* syscall: write */
    trap    #0
    bra     read_loop

exit_success:
    moveq   #1, %d0             /* syscall: exit */
    moveq   #0, %d1             /* status = 0 */
    trap    #0

exit_fail:
    moveq   #1, %d0             /* syscall: exit */
    moveq   #1, %d1             /* status = 1 */
    trap    #0

.data
filename:
    .ascii "input.txt\0"        /* adjust this to the file you want to read */

.bss
.lcomm buffer, 4096
.set BUFFER_SIZE, 4096
