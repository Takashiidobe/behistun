#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_lseek_test.txt";
  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }

  if (syscall(SYS_write, fd, "abc", 3) != 3) {
    syscall(SYS_close, fd);
    syscall(SYS_unlink, path);
    return 1;
  }

  if (syscall(SYS_lseek, fd, 0, SEEK_SET) < 0) {
    syscall(SYS_close, fd);
    syscall(SYS_unlink, path);
    return 1;
  }

  char buf[3];
  if (syscall(SYS_read, fd, buf, sizeof(buf)) != 3) {
    syscall(SYS_close, fd);
    syscall(SYS_unlink, path);
    return 1;
  }

  syscall(SYS_close, fd);
  syscall(SYS_unlink, path);
  return 0;
}
