#include <sys/syscall.h>
#include <unistd.h>

int main() {
#ifdef __NR_getresuid32
  uid_t r, e, s;
  long res = syscall(__NR_getresuid32, &r, &e, &s);
  return res == 0 ? 0 : 1;
#else
  return 0;
#endif
}
