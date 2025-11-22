#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  const char *path = "/tmp/tmp_unlinkat.txt";
  int fd = open(path, O_CREAT | O_WRONLY | O_TRUNC, 0644);
  assert(fd >= 0);
  close(fd);
  assert(unlinkat(AT_FDCWD, path, 0) == 0);
  printf("unlinked\n");
  return 0;
}
