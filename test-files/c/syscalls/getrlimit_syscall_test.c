#include <sys/resource.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct rlimit lim;
  return syscall(SYS_getrlimit, RLIMIT_NOFILE, &lim) == 0 ? 0 : 1;
}
