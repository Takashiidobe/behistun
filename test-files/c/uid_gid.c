#include <assert.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  uid_t uid = getuid();
  gid_t gid = getgid();
  assert(uid >= 0 && gid >= 0);
  printf("%u %u\n", uid, gid);
  return 0;
}
