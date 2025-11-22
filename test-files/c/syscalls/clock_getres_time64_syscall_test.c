#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts;
  syscall(SYS_clock_getres_time64, CLOCK_REALTIME, &ts);
  return 0;
}
