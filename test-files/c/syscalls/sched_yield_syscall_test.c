#include <sched.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_sched_yield);
  return 0;
}
