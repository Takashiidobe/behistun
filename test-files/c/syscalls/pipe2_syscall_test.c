#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fds[2];
  long res = syscall(SYS_pipe2, fds, O_NONBLOCK);
  if (res == 0) {
    syscall(SYS_close, fds[0]);
    syscall(SYS_close, fds[1]);
  }
  return res == 0 || res < 0 ? 0 : 1;
}
