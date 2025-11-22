#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_mkdirat_dir";
  long res = syscall(SYS_mkdirat, AT_FDCWD, path, 0700);
  if (res == 0) {
    syscall(SYS_unlinkat, AT_FDCWD, path, AT_REMOVEDIR);
  }
  return res == 0 ? 0 : 0;
}
