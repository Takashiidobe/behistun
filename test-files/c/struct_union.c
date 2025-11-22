#include <stdio.h>
#include <string.h>

struct Payload {
  int a;
  short b;
  char c;
};

union U {
  unsigned char bytes[4];
  unsigned int val;
};

int main(void) {
  struct Payload p = {.a = 0x12345678, .b = -1234, .c = 0x7f};
  int sum = p.a + p.b + p.c; /* relies on big-endian load/store correctness */

  union U u = {.val = 0xdeadbeef};
  unsigned xor = 0;
  for (int i = 0; i < 4; ++i) {
    xor^= u.bytes[i];
  }

  printf("%d %u\n", sum, xor);
  return (sum + (int)xor) & 0xff;
}
