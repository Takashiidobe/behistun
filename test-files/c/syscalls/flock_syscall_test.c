#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "flock_test", 0);
  if (fd < 0) {
    return 1;
  }

  long res = syscall(SYS_flock, fd, LOCK_EX | LOCK_UN);
  syscall(SYS_close, fd);
  return res == 0 ? 0 : 1;
}
