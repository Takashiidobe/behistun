#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_dup2_test.txt";
  int fd = syscall(SYS_open, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }

  int target = 100;
  int res = syscall(SYS_dup2, fd, target);
  if (res < 0) {
    syscall(SYS_close, fd);
    syscall(SYS_unlink, path);
    return 1;
  }

  syscall(SYS_close, res);
  syscall(SYS_close, fd);
  syscall(SYS_unlink, path);
  return 0;
}
