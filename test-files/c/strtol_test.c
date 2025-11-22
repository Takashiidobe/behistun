#include <stdio.h>
#include <stdlib.h>

int main(void) {
  // Test basic parsing
  char *endptr;
  long val = strtol("12345", &endptr, 10);
  if (val == 12345 && *endptr == '\0') {
    printf("strtol decimal ok\n");
  }

  // Test hex parsing
  val = strtol("0x1A2B", &endptr, 16);
  if (val == 0x1A2B) {
    printf("strtol hex ok\n");
  }

  // Test with auto base detection
  val = strtol("0x100", &endptr, 0);
  if (val == 256) {
    printf("strtol auto hex ok\n");
  }

  // Test negative numbers
  val = strtol("-9876", &endptr, 10);
  if (val == -9876) {
    printf("strtol negative ok\n");
  }

  // Test with trailing characters
  val = strtol("123abc", &endptr, 10);
  if (val == 123 && *endptr == 'a') {
    printf("strtol partial ok\n");
  }

  return 0;
}
