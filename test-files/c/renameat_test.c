#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  const char *old = "tmp_rename_old.txt";
  const char *newn = "tmp_rename_new.txt";
  int fd = open(old, O_CREAT | O_WRONLY | O_TRUNC, 0644);
  assert(fd >= 0);
  close(fd);
  assert(renameat(AT_FDCWD, old, AT_FDCWD, newn) == 0);
  printf("renamed\n");
  unlink(newn);
  return 0;
}
