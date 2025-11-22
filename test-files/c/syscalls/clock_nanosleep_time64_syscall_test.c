#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts = {0, 1000000};
  syscall(SYS_clock_nanosleep_time64, CLOCK_REALTIME, 0, &ts, 0);
  return 0;
}
