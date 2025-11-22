#include <sys/resource.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Try setting to current nice value; expect success or EPERM.
  long res =
      syscall(SYS_setpriority, PRIO_PROCESS, 0, getpriority(PRIO_PROCESS, 0));
  return res == 0 || res < 0 ? 0 : 1;
}
