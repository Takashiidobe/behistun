#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double x = 1.0;
  double s = sinh(x);
  double c = cosh(x);
  double t = tanh(x);
  assert(s > 1.17 && s < 1.18);
  assert(c > 1.54 && c < 1.55);
  assert(t > 0.76 && t < 0.77);
  printf("%.6f %.6f %.6f\n", s, c, t);
  return 0;
}
