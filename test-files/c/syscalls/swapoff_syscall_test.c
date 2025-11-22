#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_swapoff, "/nonexistent");
  (void)res;
  return 0;
}
