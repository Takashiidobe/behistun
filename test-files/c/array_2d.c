#include <stdio.h>

int main(void) {
  int matrix[3][3] = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};

  // Print all elements
  for (int i = 0; i < 3; i++) {
    for (int j = 0; j < 3; j++) {
      printf("%d\n", matrix[i][j]);
    }
  }

  // Calculate sum of diagonal
  int diag_sum = 0;
  for (int i = 0; i < 3; i++) {
    diag_sum += matrix[i][i];
  }
  printf("%d\n", diag_sum);

  return 0;
}
