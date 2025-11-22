#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_unlinkat_test.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, path, O_CREAT | O_RDWR, 0644);
  if (fd >= 0) {
    syscall(SYS_close, fd);
    syscall(SYS_unlinkat, AT_FDCWD, path, 0);
  }
  return 0;
}
