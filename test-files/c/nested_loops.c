#include <stdio.h>

int main(void) {
  int sum = 0;

  for (int i = 1; i <= 10; i++) {
    for (int j = 1; j <= 10; j++) {
      sum += i * j;
    }
  }

  printf("%d\n", sum);

  // Calculate sum again with different loop structure
  int sum2 = 0;
  for (int i = 1; i <= 10; i++) {
    int row_sum = 0;
    for (int j = 1; j <= 10; j++) {
      row_sum += i * j;
    }
    sum2 += row_sum;
  }

  printf("%d\n", sum2);

  return 0;
}
