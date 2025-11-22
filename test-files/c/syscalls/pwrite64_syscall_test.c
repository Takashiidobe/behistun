#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "pwrite64_test", 0);
  if (fd < 0) {
    return 1;
  }
  long res = syscall(SYS_pwrite64, fd, "data", 4, 0);
  syscall(SYS_close, fd);
  return res == 4 ? 0 : 1;
}
