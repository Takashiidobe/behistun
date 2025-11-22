#include <stdio.h>
#include <sys/time.h>

int main(void) {
  struct timeval tv;

  if (gettimeofday(&tv, NULL) != 0) {
    perror("gettimeofday");
    return 1;
  }

  // Check that we got reasonable values
  if (tv.tv_sec > 0) {
    printf("gettimeofday works\n");
  }

  if (tv.tv_usec >= 0 && tv.tv_usec < 1000000) {
    printf("microseconds valid\n");
  }

  // Call it twice and ensure time advances (or stays same)
  struct timeval tv2;
  gettimeofday(&tv2, NULL);

  if (tv2.tv_sec >= tv.tv_sec) {
    printf("time monotonic\n");
  }

  return 0;
}
