#include <assert.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  char buf[256];
  assert(gethostname(buf, sizeof(buf)) == 0);
  buf[255] = '\0';
  printf("%s\n", buf);
  return 0;
}
