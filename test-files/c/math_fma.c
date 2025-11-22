#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = fma(2.0, 3.0, 4.0); // 2*3+4 = 10
  assert(v == 10.0);
  printf("%.1f\n", v);
  return 0;
}
