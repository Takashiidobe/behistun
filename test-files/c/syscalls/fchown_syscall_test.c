#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "fchown_test", 0);
  if (fd < 0) {
    return 1;
  }

  uid_t uid = getuid();
  gid_t gid = getgid();
  if (syscall(SYS_fchown, fd, uid, gid) < 0) {
    syscall(SYS_close, fd);
    return 1;
  }

  syscall(SYS_close, fd);
  return 0;
}
