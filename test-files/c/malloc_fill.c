#include <stdio.h>
#include <stdlib.h>

int main(void) {
  size_t n = 128;
  unsigned char *p = malloc(n);
  if (!p) {
    return 1;
  }
  for (size_t i = 0; i < n; ++i) {
    p[i] = (unsigned char)(i ^ 0x5a);
  }
  unsigned int sum = 0;
  for (size_t i = 0; i < n; ++i) {
    sum += p[i];
  }
  free(p);
  printf("%u\n", sum);
  return sum & 0xff;
}
