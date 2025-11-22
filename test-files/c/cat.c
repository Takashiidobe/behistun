#include <stdio.h>
#include <stdlib.h>

int main(int argc, char *argv[]) {
  if (argc < 2) {
    fprintf(stderr, "usage: %s FILE\n", argv[0]);
    return 1;
  }

  const char *filename = argv[1];
  FILE *f = fopen(filename, "rb");
  if (!f) {
    perror("fopen");
    return 1;
  }

  unsigned char buf[4096];
  size_t n;

  while ((n = fread(buf, 1, sizeof(buf), f)) > 0) {
    if (fwrite(buf, 1, n, stdout) != n) {
      perror("fwrite");
      fclose(f);
      return 1;
    }
  }

  if (ferror(f)) {
    perror("fread");
    fclose(f);
    return 1;
  }

  fclose(f);
  return 0;
}
