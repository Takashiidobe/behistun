#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double r = remainder(7.0, 3.0); // should be 1.0 or -1.0 depending on rounding
  assert(r == 1.0 || r == -1.0);
  printf("%.1f\n", r);
  return 0;
}
