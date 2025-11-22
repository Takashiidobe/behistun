#include <sys/syscall.h>
#include <unistd.h>

int main(void) {
#ifdef __NR_setfsuid32
  uid_t uid = getuid();
  long res = syscall(__NR_setfsuid32, uid);
  return res >= 0 || res < 0 ? 0 : 1;
#else
  return 0;
#endif
}
