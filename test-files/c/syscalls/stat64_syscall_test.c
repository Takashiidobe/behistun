#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_stat64
#define SYS_stat64 SYS_stat
#endif

int main() {
  const char *path = "syscall_stat64_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  struct stat st;
  long res = syscall(SYS_stat64, path, &st);
  syscall(SYS_unlink, path);
  return res == 0 ? 0 : 1;
}
