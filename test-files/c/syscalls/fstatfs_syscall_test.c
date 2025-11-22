#include <fcntl.h>
#include <sys/statfs.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_open, ".", O_RDONLY);
  if (fd < 0) {
    return 1;
  }

  struct statfs st;
  int ok = syscall(SYS_fstatfs, fd, &st);
  syscall(SYS_close, fd);
  return ok == 0 ? 0 : 1;
}
