#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  void *p =
      mmap(0, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (p == MAP_FAILED) {
    return 1;
  }
  ((char *)p)[0] = 1;
  long res = syscall(SYS_msync, p, 4096, MS_SYNC);
  syscall(SYS_munmap, p, 4096);
  return res == 0 ? 0 : 1;
}
