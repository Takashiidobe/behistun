#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_socket, AF_UNIX, SOCK_DGRAM, 0);
  if (fd >= 0) {
    struct msghdr msg = {0};
    syscall(SYS_sendmsg, fd, &msg, 0);
    syscall(SYS_close, fd);
  }
  return 0;
}
