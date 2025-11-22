#include <stdio.h>
#include <stdlib.h>

static int cmp_int(const void *a, const void *b) {
  int ia = *(const int *)a;
  int ib = *(const int *)b;
  return (ia > ib) - (ia < ib);
}

int main(void) {
  int vals[] = {1, 3, 5, 7, 9, 11};
  size_t n = sizeof(vals) / sizeof(vals[0]);
  int key = 7;
  int *found = bsearch(&key, vals, n, sizeof(int), cmp_int);
  int idx = found ? (int)(found - vals) : -1;
  printf("%d\n", idx);
  return idx;
}
