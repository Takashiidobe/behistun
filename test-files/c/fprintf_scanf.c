#include <stdio.h>

int main(void) {
  char buf[64];
  int a = 42, b = -7;
  int n = snprintf(buf, sizeof(buf), "a=%d b=%d", a, b);
  int x = 0, y = 0;
  int scanned = sscanf(buf, "a=%d b=%d", &x, &y);
  printf("%s | %d %d %d\n", buf, n, x, y);
  return (scanned == 2) ? 0 : 1;
}
