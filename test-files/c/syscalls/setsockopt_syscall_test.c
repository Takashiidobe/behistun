#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_socket, AF_UNIX, SOCK_STREAM, 0);
  if (fd >= 0) {
    int val = 1;
    syscall(SYS_setsockopt, fd, SOL_SOCKET, SO_REUSEADDR, &val, sizeof(val));
    syscall(SYS_close, fd);
  }
  return 0;
}
