#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double s, c;
  s = sin(0.5);
  c = cos(0.5);
  assert(s > 0.47 && s < 0.49);
  assert(c > 0.87 && c < 0.88);
  printf("%.6f %.6f\n", s, c);
  return 0;
}
