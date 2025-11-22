#include <assert.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  char orig[256];
  assert(getcwd(orig, sizeof(orig)) != NULL);
  assert(chdir("examples") == 0);
  char now[256];
  assert(getcwd(now, sizeof(now)) != NULL);
  printf("%s\n%s\n", orig, now);
  assert(chdir("..") == 0);
  return 0;
}
