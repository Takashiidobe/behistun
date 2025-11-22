#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_fcntl64
#define SYS_fcntl64 SYS_fcntl
#endif

int main() {
  int fd = syscall(SYS_memfd_create, "fcntl64_test", 0);
  if (fd < 0) {
    return 1;
  }
  long res = syscall(SYS_fcntl64, fd, F_GETFL);
  syscall(SYS_close, fd);
  return res >= 0 ? 0 : 1;
}
