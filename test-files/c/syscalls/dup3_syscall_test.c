#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_dup3_test.txt";
  int fd = syscall(SYS_open, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 0;
  }
  int res = syscall(SYS_dup3, fd, 200, 0);
  syscall(SYS_close, res);
  syscall(SYS_close, fd);
  syscall(SYS_unlink, path);
  return 0;
}
