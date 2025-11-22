#include <fcntl.h>
#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_futimesat_test.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  struct timeval tv[2] = {{0, 0}, {0, 0}};
  long res = syscall(SYS_futimesat, AT_FDCWD, path, tv);
  syscall(SYS_unlink, path);
  return res == 0 || res < 0 ? 0 : 1;
}
