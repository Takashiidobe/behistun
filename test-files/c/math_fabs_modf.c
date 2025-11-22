#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = -5.25;
  double iv;
  double frac = modf(v, &iv);
  double ab = fabs(v);
  assert(iv == -5.0);
  assert(frac < 0);
  assert(ab == 5.25);
  printf("%.2f %.2f %.2f\n", iv, frac, ab);
  return 0;
}
