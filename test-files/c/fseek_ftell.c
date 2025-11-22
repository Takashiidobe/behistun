#include <stdio.h>

int main(void) {
  FILE *f = fopen("Cargo.toml", "rb");
  if (!f) {
    perror("fopen");
    return 1;
  }
  if (fseek(f, 0, SEEK_END) != 0) {
    perror("fseek");
    return 1;
  }
  long end = ftell(f);
  rewind(f);
  int ch = fgetc(f);
  fclose(f);
  printf("%ld %d\n", end, ch);
  return (int)(end & 0xff);
}
