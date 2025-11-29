#include <stdio.h>

int main(void) {
  FILE *f = tmpfile();
  if (!f) {
    perror("tmpfile");
    return 1;
  }

  if (fputs("hi\n", f) == EOF) {
    perror("fputs");
    fclose(f);
    return 1;
  }

  if (fseek(f, 0, SEEK_SET) != 0) {
    perror("fseek");
    fclose(f);
    return 1;
  }

  char buf[8] = {0};
  if (!fgets(buf, sizeof(buf), f)) {
    perror("fgets");
    fclose(f);
    return 1;
  }

  printf("%s", buf);

  fclose(f);
  return 0;
}
