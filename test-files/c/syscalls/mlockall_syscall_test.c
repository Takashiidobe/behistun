#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_mlockall, MCL_CURRENT);
  syscall(SYS_munlockall);
  return res == 0 || res < 0 ? 0 : 1;
}
