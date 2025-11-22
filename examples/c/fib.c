#include <stdint.h>
#include <stdio.h>

static uint64_t fib(uint64_t n) {
  if (n <= 1) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

int main(void) {
  uint64_t n = 15;
  uint64_t result = fib(n);
  printf("fib(%llu) = %llu\n", (unsigned long long)n,
         (unsigned long long)result);
  return 0;
}
