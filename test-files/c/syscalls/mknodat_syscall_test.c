#include <fcntl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_mknodat_fifo";
  long res = syscall(SYS_mknodat, AT_FDCWD, path, S_IFIFO | 0600, 0);
  if (res == 0) {
    syscall(SYS_unlink, path);
  }
  return 0;
}
