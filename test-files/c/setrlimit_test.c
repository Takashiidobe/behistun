#include <assert.h>
#include <stdio.h>
#include <sys/resource.h>

int main(void) {
  struct rlimit r;
  assert(getrlimit(RLIMIT_NOFILE, &r) == 0);
  printf("%lu %lu\n", (unsigned long)r.rlim_cur, (unsigned long)r.rlim_max);
  return 0;
}
