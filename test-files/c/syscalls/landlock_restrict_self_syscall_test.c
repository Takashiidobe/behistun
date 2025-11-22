#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_landlock_restrict_self, 0);
  return 0;
}
