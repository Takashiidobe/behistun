#include <stdio.h>

union data {
  int i;
  float f;
  char bytes[4];
};

int main(void) {
  union data d;

  d.i = 0x12345678;
  printf("%02x\n", (unsigned char)d.bytes[0]);
  printf("%02x\n", (unsigned char)d.bytes[1]);
  printf("%02x\n", (unsigned char)d.bytes[2]);
  printf("%02x\n", (unsigned char)d.bytes[3]);

  d.bytes[0] = 0xAB;
  d.bytes[1] = 0xCD;
  d.bytes[2] = 0xEF;
  d.bytes[3] = 0x01;

  printf("%08x\n", (unsigned int)d.i);

  return 0;
}
