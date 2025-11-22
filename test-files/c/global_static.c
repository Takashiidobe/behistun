#include <stdio.h>

int global_var = 100;
static int static_global = 200;

int counter(void) {
  static int count = 0;
  return ++count;
}

int main(void) {
  printf("%d\n", global_var);
  printf("%d\n", static_global);

  global_var += 50;
  printf("%d\n", global_var);

  printf("%d\n", counter());
  printf("%d\n", counter());
  printf("%d\n", counter());
  printf("%d\n", counter());
  printf("%d\n", counter());

  return 0;
}
