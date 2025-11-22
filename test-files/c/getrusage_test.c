#include <stdio.h>
#include <sys/resource.h>

int main(void) {
  struct rusage usage;

  // Get resource usage for current process
  if (getrusage(RUSAGE_SELF, &usage) != 0) {
    perror("getrusage");
    return 1;
  }

  printf("getrusage works\n");

  // Check that we got some reasonable values
  // (user time and system time might be 0 for a quick program)
  if (usage.ru_maxrss >= 0) {
    printf("maxrss ok\n");
  }

  // Try again after doing some work
  volatile int sum = 0;
  for (int i = 0; i < 10000; i++) {
    sum += i;
  }

  if (getrusage(RUSAGE_SELF, &usage) == 0) {
    printf("second getrusage works\n");
  }

  return 0;
}
