#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_access_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  int ok = syscall(SYS_access, path, R_OK | W_OK);
  syscall(SYS_unlink, path);
  return ok == 0 ? 0 : 1;
}
