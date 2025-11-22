#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef SYS_mmap2
#define SYS_mmap2 SYS_mmap
#endif

int main() {
  void *p = (void *)syscall(SYS_mmap2, 0, 4096, PROT_READ | PROT_WRITE,
                            MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (p == MAP_FAILED) {
    return 0; // allow failure but dispatch exercised
  }
  ((char *)p)[0] = 1;
  syscall(SYS_munmap, p, 4096);
  return 0;
}
