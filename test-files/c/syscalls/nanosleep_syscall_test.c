#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  struct timespec ts = {0, 1000000};
  return syscall(SYS_nanosleep, &ts, 0) == 0 ? 0 : 1;
}
