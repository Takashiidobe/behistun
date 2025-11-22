#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_timerfd_settime64, 0, 0, (const struct itimerspec *)0,
          (struct itimerspec *)0);
  return 0;
}
