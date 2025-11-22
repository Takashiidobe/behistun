#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *oldp = "syscall_linkat_old.txt";
  const char *newp = "syscall_linkat_new.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, oldp, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);
  syscall(SYS_linkat, AT_FDCWD, oldp, AT_FDCWD, newp, 0);
  syscall(SYS_unlink, oldp);
  syscall(SYS_unlink, newp);
  return 0;
}
