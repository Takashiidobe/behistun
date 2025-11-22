#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "fstat_test", 0);
  if (fd < 0) {
    return 1;
  }

  struct stat st;
  int ok = syscall(SYS_fstat, fd, &st);
  syscall(SYS_close, fd);
  return ok == 0 ? 0 : 1;
}
