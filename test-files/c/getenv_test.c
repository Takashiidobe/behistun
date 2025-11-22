#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
  // Just test that getenv doesn't crash
  const char *test = getenv("NONEXISTENT_VAR_12345");
  if (test == NULL) {
    printf("getenv works\n");
  } else {
    printf("unexpected result\n");
  }

  // Test getenv with another variable
  const char *test2 = getenv("ANOTHER_VAR_99999");
  if (test2 == NULL) {
    printf("getenv works again\n");
  }

  return 0;
}
