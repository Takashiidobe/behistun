#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_fchown32
  int fd = syscall(SYS_memfd_create, "fchown32_test", 0);
  if (fd < 0) {
    return 1;
  }

  uid_t uid = getuid();
  gid_t gid = getgid();
  if (syscall(__NR_fchown32, fd, uid, gid) < 0) {
    syscall(SYS_close, fd);
    return 1;
  }

  syscall(SYS_close, fd);
  return 0;
#else
  return 0;
#endif
}
