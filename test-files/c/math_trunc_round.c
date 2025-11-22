#include <assert.h>
#include <math.h>
#include <stdio.h>

int main(void) {
  double v = -2.7;
  double tr = trunc(v);
  double rd = round(v);
  double fl = floor(v);
  double ce = ceil(v);
  assert(tr == -2.0);
  assert(rd == -3.0);
  assert(fl == -3.0);
  assert(ce == -2.0);
  printf("%.1f %.1f %.1f %.1f\n", tr, rd, fl, ce);
  return 0;
}
