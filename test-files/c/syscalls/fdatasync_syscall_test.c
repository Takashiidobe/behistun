#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "fdatasync_test", 0);
  if (fd < 0) {
    return 1;
  }

  long res = syscall(SYS_fdatasync, fd);
  syscall(SYS_close, fd);
  return res == 0 ? 0 : 1;
}
