#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  int fd = open("Cargo.toml", O_RDONLY);
  assert(fd >= 0);
  int rc = posix_fadvise(fd, 0, 0, POSIX_FADV_SEQUENTIAL);
  printf("%d\n", rc);
  close(fd);
  return 0;
}
