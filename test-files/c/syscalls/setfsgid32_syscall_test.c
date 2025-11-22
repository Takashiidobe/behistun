#include <sys/syscall.h>
#include <unistd.h>

int main(void) {
#ifdef __NR_setfsgid32
  gid_t gid = getgid();
  long res = syscall(__NR_setfsgid32, gid);
  return res >= 0 || res < 0 ? 0 : 1;
#else
  return 0;
#endif
}
