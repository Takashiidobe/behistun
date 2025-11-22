#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_getuid32
  return syscall(__NR_getuid32) == getuid() ? 0 : 1;
#else
  return 0;
#endif
}
