#include <sys/eventfd.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_eventfd, 0);
  if (fd >= 0) {
    syscall(SYS_close, fd);
  }
  return 0;
}
