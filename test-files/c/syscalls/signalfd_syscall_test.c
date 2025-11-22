#include <signal.h>
#include <sys/signalfd.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  sigset_t set;
  sigemptyset(&set);
  int fd = syscall(SYS_signalfd, -1, &set, 0);
  if (fd >= 0) {
    syscall(SYS_close, fd);
  }
  return 0;
}
