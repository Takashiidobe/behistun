#include <fcntl.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_utimensat_test.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);
  struct timespec ts[2] = {{0, 0}, {0, 0}};
  syscall(SYS_utimensat, AT_FDCWD, path, ts, 0);
  syscall(SYS_unlink, path);
  return 0;
}
