#include <sys/epoll.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int epfd = syscall(SYS_epoll_create, 1);
  int fd = syscall(SYS_memfd_create, "epoll_ctl_test", 0);
  if (epfd < 0 || fd < 0) {
    if (epfd >= 0)
      syscall(SYS_close, epfd);
    if (fd >= 0)
      syscall(SYS_close, fd);
    return 0;
  }
  struct epoll_event ev = {.events = EPOLLIN, .data.fd = fd};
  long res = syscall(SYS_epoll_ctl, epfd, EPOLL_CTL_ADD, fd, &ev);
  syscall(SYS_close, fd);
  syscall(SYS_close, epfd);
  return res == 0 || res < 0 ? 0 : 1;
}
