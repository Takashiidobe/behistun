#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *path = "/tmp/syscall_link_test.txt";
  const char *link_path = "syscall_link_test_hardlink.txt";

  int fd = syscall(SYS_creat, path, 0644);
  if (fd < 0) {
    return 1;
  }
  syscall(SYS_close, fd);

  if (syscall(SYS_link, path, link_path) < 0) {
    syscall(SYS_unlink, path);
    return 1;
  }

  syscall(SYS_unlink, path);
  syscall(SYS_unlink, link_path);
  return 0;
}
