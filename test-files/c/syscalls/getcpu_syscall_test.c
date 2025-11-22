#include <sys/syscall.h>
#include <unistd.h>

int main() {
  unsigned cpu = 0, node = 0;
  long res = syscall(SYS_getcpu, &cpu, &node, 0);
  return res == 0 || res < 0 ? 0 : 1;
}
