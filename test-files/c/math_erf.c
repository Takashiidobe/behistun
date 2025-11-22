#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = erf(1.0);
  assert(v > 0.84 && v < 0.85);
  printf("%.6f\n", v);
  return 0;
}
