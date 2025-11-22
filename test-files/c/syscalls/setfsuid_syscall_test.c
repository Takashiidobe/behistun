#include <sys/syscall.h>
#include <unistd.h>

int main() {
  uid_t uid = getuid();
  long res = syscall(SYS_setfsuid, uid);
  return res >= 0 || res < 0 ? 0 : 1;
}
