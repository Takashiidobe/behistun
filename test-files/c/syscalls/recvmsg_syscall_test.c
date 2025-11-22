#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_socket, AF_UNIX, SOCK_DGRAM, 0);
  if (fd >= 0) {
    struct msghdr msg = {0};
    // Use MSG_DONTWAIT to avoid blocking when no data is available
    // This should return -1 with EAGAIN/EWOULDBLOCK
    syscall(SYS_recvmsg, fd, &msg, MSG_DONTWAIT);
    syscall(SYS_close, fd);
  }
  return 0;
}
