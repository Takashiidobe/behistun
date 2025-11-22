#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "ftruncate_test", 0);
  if (fd < 0) {
    return 1;
  }

  if (syscall(SYS_ftruncate, fd, 128) < 0) {
    syscall(SYS_close, fd);
    return 1;
  }

  syscall(SYS_close, fd);
  return 0;
}
