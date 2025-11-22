#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_mknod_test_fifo";
  if (syscall(SYS_mknod, path, S_IFIFO | 0600, 0) < 0) {
    return 1;
  }

  syscall(SYS_unlink, path);
  return 0;
}
