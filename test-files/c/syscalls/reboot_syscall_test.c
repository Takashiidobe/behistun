#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Expected to fail; ensure syscall wiring.
  long res = syscall(SYS_reboot, 0, 0, 0, 0);
  (void)res;
  return 0;
}
