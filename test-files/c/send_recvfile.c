#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/sendfile.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
  int sv[2];
  assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);

  int fd = open("Cargo.toml", O_RDONLY);
  assert(fd >= 0);

  off_t offset = 0;
  ssize_t n = sendfile(sv[0], fd, &offset, 16);
  assert(n > 0);

  char buf[32] = {0};
  assert(read(sv[1], buf, sizeof(buf)) == n);
  printf("%s\n", buf);

  close(fd);
  close(sv[0]);
  close(sv[1]);
  return 0;
}
