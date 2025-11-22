#include <stdio.h>
#include <string.h>

int main(void) {
  char buf1[] = "Hello, World!";
  char buf2[] = "Hello, World!";
  char buf3[] = "Hello, Worlz!";

  // Test equal buffers
  if (memcmp(buf1, buf2, 13) == 0) {
    printf("memcmp equal ok\n");
  }

  // Test different buffers
  if (memcmp(buf1, buf3, 13) < 0) {
    printf("memcmp less than ok\n");
  }

  // Test partial comparison
  if (memcmp(buf1, buf3, 10) == 0) {
    printf("memcmp partial ok\n");
  }

  // Test with numbers
  unsigned char nums1[] = {1, 2, 3, 4, 5};
  unsigned char nums2[] = {1, 2, 3, 4, 5};
  unsigned char nums3[] = {1, 2, 3, 4, 6};

  if (memcmp(nums1, nums2, 5) == 0) {
    printf("memcmp numbers ok\n");
  }

  if (memcmp(nums1, nums3, 5) != 0) {
    printf("memcmp numbers diff ok\n");
  }

  return 0;
}
