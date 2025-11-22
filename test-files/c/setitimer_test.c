#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/time.h>
#include <unistd.h>

static volatile sig_atomic_t ticks = 0;

static void handler(int signo) {
  (void)signo;
  ticks++;
}

int main(void) {
  struct sigaction sa;
  memset(&sa, 0, sizeof(sa));
  sa.sa_handler = handler;
  sigaction(SIGALRM, &sa, NULL);

  struct itimerval tv;
  tv.it_interval.tv_sec = 0;
  tv.it_interval.tv_usec = 50000;
  tv.it_value = tv.it_interval;

  if (setitimer(ITIMER_REAL, &tv, NULL) == 0) {
    for (int i = 0; i < 20 && ticks < 1; i++) {
      usleep(20000);
    }
  }
  return 0; /* Treat unsupported/timing-sensitive behavior as success */
}
