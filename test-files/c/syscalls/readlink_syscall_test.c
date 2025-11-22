#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *target = "syscall_readlink_target.txt";
  const char *linkpath = "syscall_readlink_link.txt";
  char buf[64];

  int fd = syscall(SYS_creat, target, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  if (syscall(SYS_symlink, target, linkpath) < 0) {
    syscall(SYS_unlink, target);
    return 1;
  }

  long len = syscall(SYS_readlink, linkpath, buf, sizeof(buf));
  syscall(SYS_unlink, linkpath);
  syscall(SYS_unlink, target);
  return len > 0 ? 0 : 1;
}
