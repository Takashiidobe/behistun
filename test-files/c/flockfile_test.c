#include <stdio.h>

int main(void) {
  FILE *f = tmpfile();
  if (!f) {
    perror("tmpfile");
    return 1;
  }

  flockfile(f);
  fputs("locked io\n", f);
  funlockfile(f);

  rewind(f);
  char buf[32] = {0};
  fgets(buf, sizeof(buf), f);
  printf("read:%s", buf);
  fclose(f);
  return 0;
}
