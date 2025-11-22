#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_socket, AF_UNIX, SOCK_DGRAM, 0);
  if (fd >= 0) {
    struct sockaddr sa = {AF_UNIX};
    syscall(SYS_sendto, fd, "x", 1, 0, &sa, sizeof(sa));
    syscall(SYS_close, fd);
  }
  return 0;
}
