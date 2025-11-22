#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_setgid32
  gid_t gid = getgid();
  return syscall(__NR_setgid32, gid) == 0 ? 0 : 1;
#else
  return 0;
#endif
}
