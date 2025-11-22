#include <stdio.h>
#include <string.h>
#include <sys/utsname.h>

int main(void) {
  struct utsname buf;

  if (uname(&buf) != 0) {
    perror("uname");
    return 1;
  }

  // Check that we got something
  if (strlen(buf.sysname) > 0) {
    printf("sysname: ok\n");
  }

  if (strlen(buf.nodename) > 0) {
    printf("nodename: ok\n");
  }

  if (strlen(buf.release) > 0) {
    printf("release: ok\n");
  }

  if (strlen(buf.machine) > 0) {
    printf("machine: ok\n");
  }

  return 0;
}
