#include <assert.h>
#include <stdio.h>
#include <sys/select.h>
#include <unistd.h>

int main(void) {
  fd_set rfds;
  FD_ZERO(&rfds);
  FD_SET(STDIN_FILENO, &rfds);
  struct timeval tv = {0, 0};
  int ret = select(STDIN_FILENO + 1, &rfds, NULL, NULL, &tv);
  assert(ret >= 0);
  printf("%d %d\n", ret, FD_ISSET(STDIN_FILENO, &rfds));
  return 0;
}
