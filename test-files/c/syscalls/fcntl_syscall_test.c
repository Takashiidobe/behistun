#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_fcntl_test.txt";
  int fd = syscall(SYS_open, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }

  int flags = syscall(SYS_fcntl, fd, F_GETFL);
  syscall(SYS_close, fd);
  syscall(SYS_unlink, path);
  return flags >= 0 ? 0 : 1;
}
