#include <dirent.h>
#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int fd = syscall(SYS_open, ".", O_RDONLY | O_DIRECTORY);
  if (fd < 0) {
    return 1;
  }

  char buf[512];
  long res = syscall(SYS_getdents, fd, buf, sizeof(buf));
  syscall(SYS_close, fd);
  return res >= 0 ? 0 : 1;
}
