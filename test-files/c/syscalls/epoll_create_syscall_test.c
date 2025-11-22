#include <sys/epoll.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_epoll_create, 1);
  if (fd < 0) {
    return 0;
  }
  syscall(SYS_close, fd);
  return 0;
}
