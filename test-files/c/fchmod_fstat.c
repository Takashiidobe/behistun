#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  const char *path = "/tmp/tmp_fchmod.txt";
  int fd = open(path, O_CREAT | O_TRUNC | O_WRONLY, 0644);
  assert(fd >= 0);
  assert(fchmod(fd, 0600) == 0);
  struct stat st;
  assert(fstat(fd, &st) == 0);
  printf("%o\n", st.st_mode & 0777);
  close(fd);
  unlink(path);
  return 0;
}
