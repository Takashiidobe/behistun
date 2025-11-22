#include <sys/select.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  fd_set set;
  FD_ZERO(&set);
  struct timespec ts = {0, 0};
  long res = syscall(SYS_pselect6, 0, &set, 0, 0, &ts, 0);
  return res >= 0 || res < 0 ? 0 : 1;
}
