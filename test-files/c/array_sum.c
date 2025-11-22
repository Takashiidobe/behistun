#include <stdio.h>

int main(void) {
  int vals[] = {1, -2, 3, 4, -5, 6, 7, -8, 9};
  long sum = 0;
  for (unsigned i = 0; i < sizeof(vals) / sizeof(vals[0]); ++i) {
    sum += vals[i];
  }
  printf("%ld\n", sum);
  return (int)(sum & 0xff);
}
