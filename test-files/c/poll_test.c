#include <assert.h>
#include <poll.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  struct pollfd pfd = {.fd = STDIN_FILENO, .events = POLLIN, .revents = 0};
  int ret = poll(&pfd, 1, 0);
  assert(ret >= 0);
  printf("%d %d\n", ret, (pfd.revents & POLLIN) != 0);
  return 0;
}
