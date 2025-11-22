#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double a = 9.0;
  double r = sqrt(a);
  double p = pow(2.0, 5.0);
  assert(r > 2.99 && r < 3.01);
  assert(p > 31.9 && p < 32.1);
  printf("%.6f %.6f\n", r, p);
  return 0;
}
