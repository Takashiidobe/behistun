#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
    syscall(SYS_timer_gettime64, 0, (struct itimerspec *)0);
    return 0;
}
