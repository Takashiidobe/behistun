#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "fallocate_test", 0);
  if (fd < 0) {
    return 0;
  }
  long res = syscall(SYS_fallocate, fd, 0, 0, 1024);
  syscall(SYS_close, fd);
  return res == 0 || res < 0 ? 0 : 1;
}
