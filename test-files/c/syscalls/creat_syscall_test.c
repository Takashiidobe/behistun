#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_creat_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }

  syscall(SYS_close, fd);
  syscall(SYS_unlink, path);
  return 0;
}
