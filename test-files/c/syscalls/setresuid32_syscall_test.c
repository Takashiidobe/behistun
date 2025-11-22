#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_setresuid32
  uid_t uid = getuid();
  long res = syscall(__NR_setresuid32, uid, uid, uid);
  return res == 0 || res < 0 ? 0 : 1;
#else
  return 0;
#endif
}
