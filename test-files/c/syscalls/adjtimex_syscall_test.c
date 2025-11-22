#include <sys/syscall.h>
#include <sys/timex.h>
#include <unistd.h>

int main() {
  struct timex tx = {0};
  // Likely to fail without privilege; dispatch is sufficient.
  long res = syscall(SYS_adjtimex, &tx);
  (void)res;
  return 0;
}
