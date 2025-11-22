#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  struct itimerval it = {{0, 0}, {0, 1000}};
  return syscall(SYS_setitimer, ITIMER_REAL, &it, 0) == 0 ? 0 : 1;
}
