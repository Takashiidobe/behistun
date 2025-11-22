#include <signal.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

int main() {
  timer_t t;
  struct sigevent sev = {.sigev_notify = SIGEV_NONE};
  if (syscall(SYS_timer_create, CLOCK_REALTIME, &sev, &t) < 0) {
    return 0;
  }
  struct itimerspec its;
  long res = syscall(SYS_timer_gettime, t, &its);
  syscall(SYS_timer_delete, t);
  return res == 0 ? 0 : 1;
}
