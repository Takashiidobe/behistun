#include <linux/futex.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int val = 0;
  long res = syscall(SYS_futex, &val, FUTEX_WAKE, 1, 0, 0, 0);
  return res >= 0 || res < 0 ? 0 : 1;
}
