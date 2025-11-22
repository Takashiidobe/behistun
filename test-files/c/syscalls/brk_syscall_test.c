#include <sys/syscall.h>
#include <unistd.h>

int main() {
  void *cur = sbrk(0);
  long res = syscall(SYS_brk, cur);
  return res == 0 ? 0 : 1;
}
