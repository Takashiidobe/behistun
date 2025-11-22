#include <sys/syscall.h>
#include <unistd.h>

int main() {
  char buf[8];
  syscall(SYS_getrandom, buf, sizeof(buf), 0);
  return 0;
}
