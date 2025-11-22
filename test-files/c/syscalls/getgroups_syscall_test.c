#include <sys/syscall.h>
#include <unistd.h>

int main() {
  gid_t groups[8];
  long res = syscall(SYS_getgroups, 8, groups);
  return res >= 0 ? 0 : 1;
}
