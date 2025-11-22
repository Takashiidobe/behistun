#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int tidaddr = 0;
  long res = syscall(SYS_set_tid_address, &tidaddr);
  return res > 0 ? 0 : 1;
}
