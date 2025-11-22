#include <stdint.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_landlock_create_ruleset, 0, 0, 0);
  return 0;
}
