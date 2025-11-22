#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = 2.5;
  double e = exp(v);
  double l = log(e);
  double l10 = log10(100.0);
  assert(l > 2.49 && l < 2.51);
  assert(l10 > 1.99 && l10 < 2.01);
  printf("%.6f %.6f %.6f\n", e, l, l10);
  return 0;
}
