#include <sys/syscall.h>
#include <sys/times.h>
#include <unistd.h>

int main() {
  struct tms buf;
  return syscall(SYS_times, &buf) >= 0 ? 0 : 1;
}
