#include <errno.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts = {0, 1000000};
  long res = syscall(SYS_clock_nanosleep, CLOCK_REALTIME, 0, &ts, 0);
  return res == 0 || res == EINTR || res < 0 ? 0 : 1;
}
