#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  int v = 0;
  // FIONREAD on stdin; may fail on some environments, but dispatch is tested.
  long res = syscall(SYS_ioctl, STDIN_FILENO, FIONREAD, &v);
  return res >= 0 || res < 0 ? 0 : 1;
}
