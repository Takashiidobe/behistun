#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = hypot(3.0, 4.0);
  assert(v > 4.99 && v < 5.01);
  printf("%.6f\n", v);
  return 0;
}
