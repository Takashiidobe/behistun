#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = tgamma(5.0); // 4! = 24
  assert(v > 23.9 && v < 24.1);
  double lg = lgamma(5.0);
  assert(lg > 3.17 && lg < 3.19);
  printf("%.6f %.6f\n", v, lg);
  return 0;
}
