#include <sys/syscall.h>
#include <unistd.h>

int main() {
  uid_t r, e, s;
  long res = syscall(SYS_getresuid, &r, &e, &s);
  return res == 0 ? 0 : 1;
}
