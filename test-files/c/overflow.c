#include <stdint.h>
#include <stdio.h>

int main(void) {
  int32_t a = INT32_MAX;
  int32_t b = -1;
  int32_t sum = a + b;              /* no overflow */
  int32_t wrap = a + 1;             /* wraps */
  uint32_t usum = 0xffffffffu + 2u; /* wraps to 1 */

  printf("%d %d %u\n", sum, wrap, usum);
  return (int)((sum ^ wrap ^ (int32_t)usum) & 0xff);
}
