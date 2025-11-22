#include <stdio.h>

int main(void) {
  const char *path = "/tmp/tmp_fmt.txt";
  FILE *f = fopen(path, "w+");
  if (!f) {
    perror("fopen");
    return 1;
  }
  fprintf(f, "x=%d y=%d\n", 10, -3);
  rewind(f);
  int x = 0, y = 0;
  if (fscanf(f, "x=%d y=%d", &x, &y) != 2) {
    perror("fscanf");
    fclose(f);
    return 1;
  }
  printf("%d %d\n", x, y);
  fclose(f);
  remove(path);
  return (x + y) & 0xff;
}
