#include <sys/syscall.h>
#include <unistd.h>

int main() {
  uid_t uid = getuid();
  return syscall(SYS_setreuid, uid, uid) == 0 ? 0 : 1;
}
