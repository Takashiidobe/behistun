#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_setreuid32
  uid_t uid = getuid();
  long res = syscall(__NR_setreuid32, uid, uid);
  return res == 0 || res < 0 ? 0 : 1;
#else
  return 0;
#endif
}
