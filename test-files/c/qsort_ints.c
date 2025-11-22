#include <stdio.h>
#include <stdlib.h>

static int cmp_int(const void *a, const void *b) {
  int ia = *(const int *)a;
  int ib = *(const int *)b;
  return (ia > ib) - (ia < ib);
}

int main(void) {
  int vals[] = {5, -1, 3, 9, 0, -7, 2};
  size_t n = sizeof(vals) / sizeof(vals[0]);
  qsort(vals, n, sizeof(vals[0]), cmp_int);
  for (size_t i = 0; i < n; ++i) {
    printf("%d ", vals[i]);
  }
  printf("\n");
  return vals[0] & 0xff;
}
