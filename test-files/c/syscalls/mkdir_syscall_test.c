#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_mkdir_test_dir";
  if (syscall(SYS_mkdir, path, 0700) < 0) {
    return 1;
  }

  if (syscall(SYS_rmdir, path) < 0) {
    return 1;
  }

  return 0;
}
