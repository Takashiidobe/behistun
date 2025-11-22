#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_fadvise64
#define SYS_fadvise64 SYS_fadvise
#endif

int main() {
  int fd = syscall(SYS_memfd_create, "fadvise64_test", 0);
  if (fd < 0) {
    return 1;
  }
  long res = syscall(SYS_fadvise64, fd, 0, 0, POSIX_FADV_NORMAL);
  syscall(SYS_close, fd);
  return res == 0 || res < 0 ? 0 : 1;
}
