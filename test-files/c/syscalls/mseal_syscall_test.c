#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_mseal, 0, 0);
  return res == -1 ? 0 : 1;
}
