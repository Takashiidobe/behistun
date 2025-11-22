#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_socket, AF_UNIX, SOCK_STREAM, 0);
  if (fd >= 0) {
    int val;
    socklen_t len = sizeof(val);
    syscall(SYS_getsockopt, fd, SOL_SOCKET, SO_TYPE, &val, &len);
    syscall(SYS_close, fd);
  }
  return 0;
}
