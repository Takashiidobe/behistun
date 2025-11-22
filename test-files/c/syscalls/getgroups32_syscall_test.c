#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_getgroups32
  gid_t groups[8];
  long res = syscall(__NR_getgroups32, 8, groups);
  return res >= 0 ? 0 : 1;
#else
  return 0;
#endif
}
