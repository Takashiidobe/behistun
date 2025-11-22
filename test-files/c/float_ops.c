#include <stdio.h>

int main(void) {
  double a = 1.5;
  double b = -2.25;
  double c = 3.0;
  double res = (a + b) * c;    /* (-0.75) * 3 = -2.25 */
  float f = (float)res / 1.5f; /* -1.5 */

  printf("%.6f %.6f\n", res, f);
  return 0;
}
