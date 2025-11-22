#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Expected to fail without privileges; dispatch is what we test.
  long res = syscall(SYS_swapon, "/nonexistent", 0);
  (void)res;
  return 0;
}
