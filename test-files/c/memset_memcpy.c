#include <stdio.h>
#include <string.h>

int main(void) {
  char buf[20];

  // Test memset
  memset(buf, 'A', 10);
  buf[10] = '\0';
  if (strcmp(buf, "AAAAAAAAAA") == 0) {
    printf("memset ok\n");
  }

  // Test memset with zero
  memset(buf, 0, sizeof(buf));
  int all_zero = 1;
  for (int i = 0; i < 20; i++) {
    if (buf[i] != 0) {
      all_zero = 0;
      break;
    }
  }
  if (all_zero) {
    printf("memset zero ok\n");
  }

  // Test memcpy
  char src[] = "Test String";
  char dst[20];
  memcpy(dst, src, strlen(src) + 1);
  if (strcmp(dst, src) == 0) {
    printf("memcpy ok\n");
  }

  // Test memcpy with numbers
  int nums_src[] = {1, 2, 3, 4, 5};
  int nums_dst[5];
  memcpy(nums_dst, nums_src, sizeof(nums_src));
  if (nums_dst[0] == 1 && nums_dst[4] == 5) {
    printf("memcpy numbers ok\n");
  }

  return 0;
}
