#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_syslog, 0, 0, 0);
  (void)res;
  return 0;
}
