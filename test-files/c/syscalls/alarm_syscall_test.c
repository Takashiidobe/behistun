#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_alarm, 1);
  return syscall(SYS_pause) < 0 ? 0 : 1;
}
