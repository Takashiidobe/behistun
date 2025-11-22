#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_fstat64
#define SYS_fstat64 SYS_fstat
#endif

int main() {
  int fd = syscall(SYS_memfd_create, "fstat64_test", 0);
  if (fd < 0) {
    return 1;
  }

  struct stat st;
  long res = syscall(SYS_fstat64, fd, &st);
  syscall(SYS_close, fd);
  return res == 0 ? 0 : 1;
}
