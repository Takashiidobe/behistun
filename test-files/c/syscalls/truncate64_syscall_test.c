#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_truncate64
#define SYS_truncate64 SYS_truncate
#endif

int main() {
  const char *path = "syscall_truncate64_test.txt";
  int fd = syscall(SYS_open, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  long res = syscall(SYS_truncate64, path, 0);
  syscall(SYS_unlink, path);
  return res == 0 ? 0 : 1;
}
