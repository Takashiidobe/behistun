#include <fcntl.h>
#include <stdio.h>
#include <sys/file.h>
#include <unistd.h>

int main(void) {
  int fd = open("Cargo.toml", O_RDONLY);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  if (flock(fd, LOCK_SH) != 0) {
    perror("flock");
    close(fd);
    return 1;
  }
  printf("locked\n");
  flock(fd, LOCK_UN);
  close(fd);
  return 0;
}
