#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
  int *arr = malloc(5 * sizeof(int));
  if (!arr) {
    printf("malloc failed\n");
    return 1;
  }

  for (int i = 0; i < 5; i++) {
    arr[i] = i * 10;
  }

  for (int i = 0; i < 5; i++) {
    printf("%d\n", arr[i]);
  }

  free(arr);

  return 0;
}
