#include <sys/syscall.h>
#include <unistd.h>
#include <utime.h>

int main() {
  const char *path = "/tmp/syscall_utime_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  struct utimbuf times = {0, 0};
  if (syscall(SYS_utime, path, &times) < 0) {
    syscall(SYS_unlink, path);
    return 1;
  }

  syscall(SYS_unlink, path);
  return 0;
}
