#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_read_test.txt";
  const char *data = "read syscall\n";
  char buf[32] = {0};

  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }

  if (syscall(SYS_write, fd, data, 13) != 13) {
    return 1;
  }
  syscall(SYS_close, fd);

  fd = syscall(SYS_open, path, O_RDONLY);
  if (fd < 0) {
    return 1;
  }

  if (syscall(SYS_read, fd, buf, sizeof(buf)) <= 0) {
    return 1;
  }

  syscall(SYS_close, fd);
  syscall(SYS_unlink, path);
  return 0;
}
