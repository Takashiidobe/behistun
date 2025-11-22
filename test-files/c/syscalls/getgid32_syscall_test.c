#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_getgid32
  return syscall(__NR_getgid32) == getgid() ? 0 : 1;
#else
  return 0;
#endif
}
