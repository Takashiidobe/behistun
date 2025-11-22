#include <sys/syscall.h>
#include <sys/timerfd.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_timerfd_create, CLOCK_MONOTONIC, 0);
  if (fd >= 0) {
    syscall(SYS_close, fd);
  }
  return 0;
}
