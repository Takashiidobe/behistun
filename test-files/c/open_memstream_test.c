#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
  char *buf = NULL;
  size_t len = 0;
  FILE *f = open_memstream(&buf, &len);
  if (!f) {
    perror("open_memstream");
    return 1;
  }

  fputs("hello", f);
  fflush(f);
  fprintf(f, " %s", "world");
  fclose(f);

  printf("len=%zu buf=%s\n", len, buf ? buf : "(null)");
  free(buf);
  return 0;
}
