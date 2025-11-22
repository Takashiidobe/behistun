#include <sys/syscall.h>
#include <unistd.h>

int main() {
  gid_t groups[1] = {getgid()};
  // Likely fails without privilege; still ensures dispatch.
  return syscall(SYS_setgroups, 1, groups) < 0 ? 0 : 1;
}
