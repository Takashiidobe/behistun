#include <sys/syscall.h>
#include <unistd.h>

int main() {
  gid_t r, e, s;
  long res = syscall(SYS_getresgid, &r, &e, &s);
  return res == 0 ? 0 : 1;
}
