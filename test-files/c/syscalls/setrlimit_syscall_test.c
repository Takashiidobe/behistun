#include <sys/resource.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct rlimit lim;
  if (syscall(SYS_getrlimit, RLIMIT_NOFILE, &lim) < 0) {
    return 1;
  }

  // Attempt to re-set to the same limits.
  return syscall(SYS_setrlimit, RLIMIT_NOFILE, &lim) == 0 ? 0 : 1;
}
