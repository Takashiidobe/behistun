#include <math.h>
#include <stdio.h>
#include <stdlib.h>

static void show(const char *s) {
  char *end = NULL;
  double v = strtod(s, &end);
  printf("%s -> value=%g consumed=%ld isnan=%d isinf=%d\n", s, v,
         end ? (long)(end - s) : -1L, isnan(v), isinf(v));
}

int main(void) {
  show("nan");
  show("+inf");
  show("-infinity");
  show("0x1.8p1");
  show("1.0e-308");
  show("  42.5junk");
  show("0x1p-2000"); /* underflow to zero */
  return 0;
}
