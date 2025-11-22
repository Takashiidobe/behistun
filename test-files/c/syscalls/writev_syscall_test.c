#include <sys/syscall.h>
#include <sys/uio.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "writev_test", 0);
  if (fd < 0) {
    return 1;
  }

  const char a[] = "hi";
  const char b[] = "!";
  struct iovec iov[2] = {{(void *)a, 2}, {(void *)b, 1}};
  long res = syscall(SYS_writev, fd, iov, 2);
  syscall(SYS_close, fd);
  return res >= 0 ? 0 : 1;
}
