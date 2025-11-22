#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_fchownat_test.txt";
  int fd = syscall(SYS_openat, AT_FDCWD, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);
  long res = syscall(SYS_fchownat, AT_FDCWD, path, getuid(), getgid(), 0);
  syscall(SYS_unlink, path);
  return res == 0 || res < 0 ? 0 : 1;
}
