#include <assert.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  mode_t old = umask(0022);
  printf("%o\n", (unsigned)old);
  const char *path = "/tmp/tmp_umask.txt";
  FILE *f = fopen(path, "w");
  assert(f);
  fclose(f);
  struct stat st;
  assert(stat(path, &st) == 0);
  printf("%o\n", (unsigned)(st.st_mode & 0777));
  unlink(path);
  return 0;
}
