#define _GNU_SOURCE
#include <stdio.h>
#include <time.h>

int main(void) {
  struct timespec ts;
  int rc = clock_getres(CLOCK_REALTIME, &ts);
  printf("clock_getres rc=%d sec=%ld\n", rc, (long)ts.tv_sec);
  return rc;
}
