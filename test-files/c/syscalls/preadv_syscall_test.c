#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "preadv_test", 0);
  if (fd < 0)
    return 0;
  syscall(SYS_write, fd, "abc", 3);
  syscall(SYS_lseek, fd, 0, SEEK_SET);
  struct iovec iov = {(void *)((char[3]){0}), 3};
  syscall(SYS_preadv, fd, &iov, 1, 0);
  syscall(SYS_close, fd);
  return 0;
}
