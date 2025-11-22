#include <assert.h>
#include <stdio.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
  int sv[2];
  assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
  const char *msg = "sockpair";
  assert(write(sv[0], msg, 8) == 8);
  char buf[16] = {0};
  assert(read(sv[1], buf, sizeof(buf)) == 8);
  printf("%s\n", buf);
  close(sv[0]);
  close(sv[1]);
  return 0;
}
