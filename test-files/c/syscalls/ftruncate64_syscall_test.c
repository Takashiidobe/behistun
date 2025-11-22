#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_ftruncate64
#define SYS_ftruncate64 SYS_ftruncate
#endif

int main() {
  int fd = syscall(SYS_memfd_create, "ftruncate64_test", 0);
  if (fd < 0) {
    return 1;
  }

  long res = syscall(SYS_ftruncate64, fd, 256);
  syscall(SYS_close, fd);
  return res == 0 ? 0 : 1;
}
