#include <sys/resource.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_getpriority, PRIO_PROCESS, 0);
  return res >= -20 ? 0 : 1;
}
