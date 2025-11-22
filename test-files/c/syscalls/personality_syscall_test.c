#include <sys/personality.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long res = syscall(SYS_personality, PER_LINUX);
  return res >= 0 || res < 0 ? 0 : 1;
}
