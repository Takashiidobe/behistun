#include <stdio.h>

int main(void) {
  int x = 42;
  unsigned int y = 255;
  long z = -1234567890L;

  printf("%d\n", x);
  printf("%u\n", y);
  printf("%ld\n", z);
  printf("%x\n", y);
  printf("%s\n", "hello");

  return 0;
}
