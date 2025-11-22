#include <signal.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  timer_t t;
  struct sigevent sev = {.sigev_notify = SIGEV_NONE};
  long res = syscall(SYS_timer_create, CLOCK_REALTIME, &sev, &t);
  if (res == 0) {
    syscall(SYS_timer_delete, t);
  }
  return res == 0 || res < 0 ? 0 : 1;
}
