#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_getegid32
  return syscall(__NR_getegid32) == getegid() ? 0 : 1;
#else
  return 0;
#endif
}
