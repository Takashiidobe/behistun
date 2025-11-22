#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *target = "syscall_symlinkat_target.txt";
  const char *linkp = "syscall_symlinkat_link.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, target, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);
  syscall(SYS_symlinkat, target, AT_FDCWD, linkp);
  syscall(SYS_unlink, target);
  syscall(SYS_unlink, linkp);
  return 0;
}
