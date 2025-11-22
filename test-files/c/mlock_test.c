#include <stdio.h>
#include <sys/mman.h>

int main(void) {
  char buf[4096];
  if (mlock(buf, sizeof(buf)) != 0) {
    perror("mlock");
    return 1;
  }
  buf[0] = 'x';
  munlock(buf, sizeof(buf));
  printf("mlock ok\n");
  return 0;
}
