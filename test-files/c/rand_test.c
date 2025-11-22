#include <stdio.h>
#include <stdlib.h>
#include <time.h>

int main(void) {
  srand(12345);
  int vals[5];
  for (int i = 0; i < 5; ++i) {
    vals[i] = rand();
  }
  printf("%d %d %d %d %d\n", vals[0], vals[1], vals[2], vals[3], vals[4]);
  return 0;
}
