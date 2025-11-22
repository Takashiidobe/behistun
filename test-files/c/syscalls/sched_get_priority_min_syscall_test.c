#include <sched.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_sched_get_priority_min, SCHED_OTHER);
  return res >= 0 ? 0 : 1;
}
