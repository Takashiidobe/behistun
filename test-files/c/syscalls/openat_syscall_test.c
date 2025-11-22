#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_openat, AT_FDCWD, "syscall_openat_test.txt",
                   O_CREAT | O_RDWR, 0644);
  if (fd >= 0) {
    syscall(SYS_close, fd);
    syscall(SYS_unlink, "syscall_openat_test.txt");
  }
  return fd >= 0 ? 0 : 1;
}
