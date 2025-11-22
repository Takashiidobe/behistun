#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts;
  return syscall(SYS_clock_gettime, CLOCK_REALTIME, &ts) == 0 ? 0 : 1;
}
