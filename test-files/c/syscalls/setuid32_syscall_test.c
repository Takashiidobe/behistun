#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_setuid32
  uid_t uid = getuid();
  return syscall(__NR_setuid32, uid) == 0 ? 0 : 1;
#else
  return 0;
#endif
}
