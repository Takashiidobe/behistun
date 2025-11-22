#include <stdarg.h>
#include <stdio.h>

int sum_ints(int count, ...) {
  va_list args;
  va_start(args, count);

  int sum = 0;
  for (int i = 0; i < count; i++) {
    sum += va_arg(args, int);
  }

  va_end(args);
  return sum;
}

int main(void) {
  printf("%d\n", sum_ints(3, 10, 20, 30));
  printf("%d\n", sum_ints(5, 1, 2, 3, 4, 5));
  printf("%d\n", sum_ints(1, 100));

  return 0;
}
