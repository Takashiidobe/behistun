#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fds[2];
  if (syscall(SYS_pipe, fds) < 0) {
    return 1;
  }

  syscall(SYS_close, fds[0]);
  syscall(SYS_close, fds[1]);
  return 0;
}
