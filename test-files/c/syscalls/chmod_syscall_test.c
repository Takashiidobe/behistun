#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_chmod_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  if (syscall(SYS_chmod, path, 0600) < 0) {
    syscall(SYS_unlink, path);
    return 1;
  }

  syscall(SYS_unlink, path);
  return 0;
}
