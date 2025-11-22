#define _GNU_SOURCE
#include <errno.h>
#include <stdio.h>
#include <string.h>

int main(void) {
  char buf[64];
  int err = ENOENT;

#if defined(__GLIBC__)
  char *msg = strerror_r(err, buf, sizeof(buf));
  printf("gnu:%s\n", msg ? msg : "(null)");
#else
  int rc = strerror_r(err, buf, sizeof(buf));
  if (rc == 0) {
    printf("xsi:%s\n", buf);
  } else {
    printf("xsi_error:%d\n", rc);
  }
#endif
  return 0;
}
