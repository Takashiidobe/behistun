#include <stdio.h>
#include <stdlib.h>

static int cleanup_called = 0;

void cleanup1(void) {
  // Note: We can't print here reliably as stdio may be closed
  cleanup_called = 1;
}

void cleanup2(void) { cleanup_called = 2; }

int main(void) {
  // Register cleanup functions
  if (atexit(cleanup1) != 0) {
    printf("atexit failed\n");
    return 1;
  }

  printf("atexit works\n");

  if (atexit(cleanup2) != 0) {
    printf("second atexit failed\n");
    return 1;
  }

  printf("multiple atexit works\n");

  // Functions will be called in reverse order on exit
  return 0;
}
