#include <assert.h>
#include <stdio.h>
#include <sys/mman.h>
#include <unistd.h>

int main(void) {
  size_t len = 4096;
  void *p = mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS,
                 -1, 0);
  assert(p != MAP_FAILED);
  ((char *)p)[0] = 'a';
  assert(mprotect(p, len, PROT_READ) == 0);
  printf("%c\n", ((char *)p)[0]);
  munmap(p, len);
  return 0;
}
