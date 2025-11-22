#include <assert.h>
#include <stdint.h>
#include <time.h>

int main(void) {
  time_t t = time(NULL);
  assert((intmax_t)t > 1760000000 &&
         (intmax_t)t < 2000000000); // arbitrary timestamp range
  return 0;
}
