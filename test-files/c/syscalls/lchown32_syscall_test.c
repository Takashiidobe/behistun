#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_lchown32
  const char *path = "/tmp/syscall_lchown32_link.txt";
  const char *target = "syscall_lchown32_target.txt";

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
  long res = syscall(__NR_lchown32, path, uid, gid);

  syscall(SYS_unlink, path);
  syscall(SYS_unlink, target);
  return res == 0 || res < 0 ? 0 : 1;
#else
  return 0;
#endif
}
