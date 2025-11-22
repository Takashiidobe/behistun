#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_setgroups32
  gid_t groups[1] = {getgid()};
  return syscall(__NR_setgroups32, 1, groups) < 0 ? 0 : 1;
#else
  return 0;
#endif
}
