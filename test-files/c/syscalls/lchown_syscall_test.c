#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_lchown_link.txt";
  const char *target = "syscall_lchown_target.txt";

  int fd = syscall(SYS_creat, target, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  if (syscall(SYS_symlink, target, path) < 0) {
    syscall(SYS_unlink, target);
    return 1;
  }

  uid_t uid = getuid();
  gid_t gid = getgid();
  long res = syscall(SYS_lchown, path, uid, gid);

  syscall(SYS_unlink, path);
  syscall(SYS_unlink, target);
  return res == 0 || res < 0 ? 0 : 1;
}
