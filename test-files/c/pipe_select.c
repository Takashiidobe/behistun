#include <assert.h>
#include <stdio.h>
#include <sys/select.h>
#include <unistd.h>

int main(void) {
  int fds[2];
  assert(pipe(fds) == 0);
  const char *msg = "pipe";
  write(fds[1], msg, 4);

  fd_set rfds;
  FD_ZERO(&rfds);
  FD_SET(fds[0], &rfds);
  int ret = select(fds[0] + 1, &rfds, NULL, NULL, NULL);
  assert(ret == 1);
  char buf[8] = {0};
  read(fds[0], buf, sizeof(buf));
  printf("%s\n", buf);

  close(fds[0]);
  close(fds[1]);
  return 0;
}
