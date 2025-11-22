#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts = {0, 0};
  long res = syscall(SYS_clock_settime, CLOCK_REALTIME, &ts);
  return res == 0 || res < 0 ? 0 : 1;
}
