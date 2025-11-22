#include <stdio.h>
#include <string.h>

int main(void) {
  FILE *f = fopen("/tmp/test_m68k.txt", "w");
  if (!f) {
    perror("fopen write");
    return 1;
  }

  fprintf(f, "line1\n");
  fprintf(f, "line2\n");
  fprintf(f, "line3\n");
  fclose(f);

  f = fopen("/tmp/test_m68k.txt", "r");
  if (!f) {
    perror("fopen read");
    return 1;
  }

  char buf[128];
  while (fgets(buf, sizeof(buf), f)) {
    printf("%s", buf);
  }

  fclose(f);
  return 0;
}
