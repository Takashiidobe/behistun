#include <stdio.h>

int add(int a, int b) { return a + b; }
int sub(int a, int b) { return a - b; }
int mul(int a, int b) { return a * b; }

int apply(int (*fn)(int, int), int a, int b) { return fn(a, b); }

int main(void) {
  printf("%d\n", apply(add, 10, 20));
  printf("%d\n", apply(sub, 30, 5));
  printf("%d\n", apply(mul, 7, 8));

  int (*op)(int, int);
  op = add;
  printf("%d\n", op(100, 50));

  op = mul;
  printf("%d\n", op(12, 3));

  return 0;
}
