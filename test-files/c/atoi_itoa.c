#include <stdio.h>
#include <stdlib.h>

int main(void) {
  const char *s = "12345";
  int v = atoi(s);
  char buf[16];
  int n = snprintf(buf, sizeof(buf), "%d", v + 10);
  printf("%d %s %d\n", v, buf, n);
  return (v + n) & 0xff;
}
