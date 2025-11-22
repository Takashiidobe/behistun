#include <fcntl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_fstatat64
#define SYS_fstatat64 SYS_newfstatat
#endif

int main() {
  const char *path = "/tmp/syscall_fstatat64_test.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);
  struct stat st;
  long res = syscall(SYS_fstatat64, AT_FDCWD, path, &st, 0);
  syscall(SYS_unlink, path);
  return res == 0 ? 0 : 1;
}
