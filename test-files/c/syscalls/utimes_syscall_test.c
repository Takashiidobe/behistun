#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  const char *path = "syscall_utimes_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  struct timeval tv[2] = {{0, 0}, {0, 0}};
  long res = syscall(SYS_utimes, path, tv);
  syscall(SYS_unlink, path);
  return res == 0 ? 0 : 1;
}
