#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
    struct itimerspec its = {{0, 0}, {0, 0}};
    syscall(SYS_timer_settime64, 0, 0, &its, 0);
    return 0;
}
