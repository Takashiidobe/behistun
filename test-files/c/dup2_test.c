#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  int fd = open("Cargo.toml", O_RDONLY);
  assert(fd >= 0);
  int newfd = 10;
  assert(dup2(fd, newfd) == newfd);
  char buf[8] = {0};
  read(newfd, buf, sizeof(buf));
  printf("%.4s\n", buf);
  close(fd);
  close(newfd);
  return 0;
}
