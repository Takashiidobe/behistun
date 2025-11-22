#include <poll.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct pollfd pfd = {.fd = -1, .events = 0};
  int res = syscall(SYS_poll, &pfd, 0, 0);
  return res >= 0 ? 0 : 1;
}
