#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "fchmod_test", 0);
  if (fd < 0) {
    return 1;
  }

  if (syscall(SYS_fchmod, fd, 0600) < 0) {
    syscall(SYS_close, fd);
    return 1;
  }

  syscall(SYS_close, fd);
  return 0;
}
