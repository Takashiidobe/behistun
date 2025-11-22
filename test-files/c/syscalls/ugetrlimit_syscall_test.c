#include <sys/resource.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct rlimit lim;
  long res = syscall(SYS_ugetrlimit, &lim);
  return res == 0 || res < 0 ? 0 : 1;
}
