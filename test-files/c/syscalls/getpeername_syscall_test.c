#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_socket, AF_UNIX, SOCK_STREAM, 0);
  if (fd >= 0) {
    struct sockaddr sa;
    socklen_t len = sizeof(sa);
    syscall(SYS_getpeername, fd, &sa, &len);
    syscall(SYS_close, fd);
  }
  return 0;
}
