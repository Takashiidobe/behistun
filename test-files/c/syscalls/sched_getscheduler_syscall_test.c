#include <sched.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_sched_getscheduler, 0);
  return res >= 0 ? 0 : 1;
}
