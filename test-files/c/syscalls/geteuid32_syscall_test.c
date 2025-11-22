#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_geteuid32
  return syscall(__NR_geteuid32) == geteuid() ? 0 : 1;
#else
  return 0;
#endif
}
