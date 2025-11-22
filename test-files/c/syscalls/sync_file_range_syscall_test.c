#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_memfd_create, "sync_file_range_test", 0);
  if (fd < 0) {
    return 0;
  }
  long res = syscall(SYS_sync_file_range, fd, 0, 0, 0);
  syscall(SYS_close, fd);
  return res == 0 || res < 0 ? 0 : 1;
}
