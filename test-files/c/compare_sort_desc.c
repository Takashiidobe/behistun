#include <stdio.h>
#include <stdlib.h>

static int cmp_desc(const void *a, const void *b) {
  int ia = *(const int *)a;
  int ib = *(const int *)b;
  return (ib > ia) - (ib < ia);
}

int main(void) {
  int vals[] = {1, 3, 2, 5, 4};
  size_t n = sizeof(vals) / sizeof(vals[0]);
  qsort(vals, n, sizeof(int), cmp_desc);
  for (size_t i = 0; i < n; ++i) {
    printf("%d ", vals[i]);
  }
  printf("\n");
  return vals[0];
}
