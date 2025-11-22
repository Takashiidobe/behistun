#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  struct itimerval it;
  return syscall(SYS_getitimer, ITIMER_REAL, &it) == 0 ? 0 : 1;
}
