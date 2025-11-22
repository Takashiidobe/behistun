#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_open, ".", O_RDONLY);
  if (fd >= 0) {
    syscall(SYS_syncfs, fd);
    syscall(SYS_close, fd);
  }
  return 0;
}
