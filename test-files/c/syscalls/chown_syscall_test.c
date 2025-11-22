#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_chown_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  uid_t uid = getuid();
  gid_t gid = getgid();

  if (syscall(SYS_chown, path, uid, gid) < 0) {
    syscall(SYS_unlink, path);
    return 1;
  }

  syscall(SYS_unlink, path);
  return 0;
}
