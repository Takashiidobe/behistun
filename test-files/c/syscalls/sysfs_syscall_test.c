#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_sysfs, 1, 0);
  (void)res;
  return 0;
}
