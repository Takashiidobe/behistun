#include <sys/resource.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct rusage ru;
  return syscall(SYS_getrusage, RUSAGE_SELF, &ru) == 0 ? 0 : 1;
}
