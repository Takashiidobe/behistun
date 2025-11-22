#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_open, ".", O_RDONLY);
  if (fd < 0) {
    return 1;
  }

  long res = syscall(SYS_fchdir, fd);
  syscall(SYS_close, fd);
  return res == 0 ? 0 : 1;
}
