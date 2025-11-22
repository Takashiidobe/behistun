#include <stdio.h>
#include <string.h>

int main(void) {
  char buf[64];

  strcpy(buf, "hello");
  printf("%s\n", buf);

  strcpy(buf, "world!");
  printf("%s\n", buf);

  strcpy(buf, "");
  printf("%s\n", buf);

  return 0;
}
