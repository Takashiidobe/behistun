#include <math.h>
#include <stdio.h>

int main(void) {
  double x = -3.5;
  double y = 2.0;
  double res = fabs(x) + pow(y, 3); /* 3.5 + 8 = 11.5 */
  double s = sin(0.0);
  printf("%.6f %.6f\n", res, s);
  return (int)res;
}
