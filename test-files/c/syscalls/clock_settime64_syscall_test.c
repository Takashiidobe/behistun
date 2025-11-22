#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts = {0, 0};
  syscall(SYS_clock_settime64, CLOCK_REALTIME, &ts);
  return 0;
}
