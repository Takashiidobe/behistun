#include <sys/syscall.h>
#include <unistd.h>

int main() {
  gid_t gid = getgid();
  return syscall(SYS_setregid, gid, gid) == 0 ? 0 : 1;
}
