#include <sys/syscall.h>
#include <unistd.h>

int main() {
  gid_t gid = getgid();
  long res = syscall(SYS_setresgid, gid, gid, gid);
  return res == 0 || res < 0 ? 0 : 1;
}
