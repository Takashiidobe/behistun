#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *oldp = "syscall_renameat_old.txt";
  const char *newp = "syscall_renameat_new.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, oldp, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);
  syscall(SYS_renameat, AT_FDCWD, oldp, AT_FDCWD, newp);
  syscall(SYS_unlink, newp);
  return 0;
}
