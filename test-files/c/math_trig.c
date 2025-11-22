#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double x = M_PI / 6.0; // 30 degrees
  double s = sin(x);
  double c = cos(x);
  double t = tan(x);
  assert(s > 0.49 && s < 0.51);
  assert(c > 0.86 && c < 0.88);
  assert(t > 0.57 && t < 0.59);
  printf("%.6f %.6f %.6f\n", s, c, t);
  return 0;
}
