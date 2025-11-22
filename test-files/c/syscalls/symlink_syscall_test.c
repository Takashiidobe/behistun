#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *target = "syscall_symlink_target.txt";
  const char *linkpath = "syscall_symlink_link.txt";

  int fd = syscall(SYS_creat, target, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  if (syscall(SYS_symlink, target, linkpath) < 0) {
    syscall(SYS_unlink, target);
    return 1;
  }

  syscall(SYS_unlink, linkpath);
  syscall(SYS_unlink, target);
  return 0;
}
