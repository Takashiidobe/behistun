#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
  void *ptr = NULL;
  int rc = posix_memalign(&ptr, 64, 1024);
  if (rc != 0 || !ptr) {
    return 1;
  }
  size_t mod = (size_t)((uintptr_t)ptr % 64);
  free(ptr);
  return mod == 0 ? 0 : 1;
}
