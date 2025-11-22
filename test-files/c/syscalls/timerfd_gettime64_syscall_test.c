#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_timerfd_gettime64, 0, (struct itimerspec *)0);
  return 0;
}
