#include <dirent.h>
#include <fcntl.h>
#include <stdint.h>
#include <sys/syscall.h>
#include <unistd.h>

struct linux_dirent64 {
  uint64_t d_ino;
  int64_t d_off;
  unsigned short d_reclen;
  unsigned char d_type;
  char d_name[];
};

int main() {
  int fd = syscall(SYS_open, ".", O_RDONLY | O_DIRECTORY);
  if (fd < 0) {
    return 1;
  }
  char buf[512];
  long n = syscall(SYS_getdents64, fd, buf, sizeof(buf));
  syscall(SYS_close, fd);
  return n >= 0 ? 0 : 1;
}
