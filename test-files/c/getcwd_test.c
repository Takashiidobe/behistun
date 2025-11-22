#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
  char buf[PATH_MAX];

  if (getcwd(buf, sizeof(buf)) == NULL) {
    perror("getcwd");
    return 1;
  }

  // Check that we got something
  if (strlen(buf) > 0) {
    printf("getcwd works\n");
  }

  // Check that it starts with /
  if (buf[0] == '/') {
    printf("path is absolute\n");
  }

  return 0;
}
