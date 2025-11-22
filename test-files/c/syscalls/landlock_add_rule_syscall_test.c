#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_landlock_add_rule, -1, 0, 0);
  return 0;
}
