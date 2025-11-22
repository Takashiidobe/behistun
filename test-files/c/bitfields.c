#include <stdio.h>

struct Flags {
  unsigned a : 3;
  unsigned b : 5;
  unsigned c : 8;
};

int main(void) {
  struct Flags f = {.a = 0b101, .b = 0x1f, .c = 0xaa};
  printf("%u %u %u\n", f.a, f.b, f.c);
  return (f.a + f.b + f.c) & 0xff;
}
