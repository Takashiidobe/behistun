#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  struct timeval tv = {0, 0};
  // Expected to fail without privileges; still exercises dispatch.
  return syscall(SYS_settimeofday, &tv, 0) < 0 ? 0 : 1;
}
