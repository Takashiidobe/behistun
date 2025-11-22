#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_close_test.txt";
  int fd = syscall(SYS_open, path, O_CREAT | O_RDWR, 0644);
  if (fd < 0) {
    return 1;
  }

  if (syscall(SYS_close, fd) < 0) {
    return 1;
  }

  syscall(SYS_unlink, path);
  return 0;
}
