#include <assert.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  char buf[256];
  ssize_t n = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
  assert(n > 0);
  buf[n] = '\0';
  printf("%s\n", buf);
  return 0;
}
