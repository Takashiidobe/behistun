#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_setresgid32
  gid_t gid = getgid();
  long res = syscall(__NR_setresgid32, gid, gid, gid);
  return res == 0 || res < 0 ? 0 : 1;
#else
  return 0;
#endif
}
