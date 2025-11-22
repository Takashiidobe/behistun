#include <fcntl.h>
#include <stdio.h>
#include <sys/statfs.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_fstatfs64
#define SYS_fstatfs64 SYS_fstatfs
#endif

int main() {
  int fd = open(".", O_RDONLY);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  struct statfs st;
  long res = syscall(SYS_fstatfs64, fd, &st);
  close(fd);
  return res;
}
