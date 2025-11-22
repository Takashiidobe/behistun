#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "memfd_test", 0);
  if (fd >= 0)
    syscall(SYS_close, fd);
  return 0;
}
