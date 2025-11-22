#include <assert.h>
#include <stdio.h>
#include <sys/vfs.h>

int main(void) {
  struct statfs s;
  assert(statfs(".", &s) == 0);
  printf("%lx %lx\n", (unsigned long)s.f_type, (unsigned long)s.f_bsize);
  return 0;
}
