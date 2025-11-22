#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "readv_test", 0);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_write, fd, "hello", 5);
  syscall(SYS_lseek, fd, 0, SEEK_SET);

  char buf1[3], buf2[3];
  struct iovec iov[2] = {{buf1, 3}, {buf2, 3}};
  long res = syscall(SYS_readv, fd, iov, 2);
  syscall(SYS_close, fd);
  return res >= 0 ? 0 : 1;
}
