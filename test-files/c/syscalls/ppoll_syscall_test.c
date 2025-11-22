#include <poll.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct pollfd pfd = {.fd = -1, .events = 0};
  struct timespec ts = {0, 0};
  long res = syscall(SYS_ppoll, &pfd, 0, &ts, 0, 0);
  return res >= 0 || res < 0 ? 0 : 1;
}
