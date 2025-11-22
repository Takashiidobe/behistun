#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "pwritev_test", 0);
  if (fd < 0)
    return 0;
  const char a[] = "hi";
  struct iovec iov = {(void *)a, 2};
  syscall(SYS_pwritev, fd, &iov, 1, 0);
  syscall(SYS_close, fd);
  return 0;
}
