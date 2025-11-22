#include <stdio.h>

int factorial(int n) {
  if (n <= 1)
    return 1;
  return n * factorial(n - 1);
}

int fibonacci(int n) {
  if (n <= 1)
    return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

int main(void) {
  printf("%d\n", factorial(5));
  printf("%d\n", factorial(10));
  printf("%d\n", fibonacci(10));

  return 0;
}
