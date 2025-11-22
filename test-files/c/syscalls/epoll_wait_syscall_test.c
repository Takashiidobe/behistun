#include <sys/epoll.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int epfd = syscall(SYS_epoll_create, 1);
  if (epfd < 0) {
    return 0;
  }
  struct epoll_event ev;
  long res = syscall(SYS_epoll_wait, epfd, &ev, 1, 0);
  syscall(SYS_close, epfd);
  return res >= 0 || res < 0 ? 0 : 1;
}
