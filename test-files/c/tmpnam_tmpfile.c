#include <stdio.h>

int main(void) {
  char name[L_tmpnam];
  if (!tmpnam(name)) {
    puts("tmpnam failed");
    return 1;
  }
  FILE *f = tmpfile();
  if (!f) {
    perror("tmpfile");
    return 1;
  }
  fputs("hi\n", f);
  rewind(f);
  char buf[8] = {0};
  fgets(buf, sizeof(buf), f);
  printf("%s\n", buf);
  return 0;
}
