#include <sys/syscall.h>
#include <sys/time.h>
#include <unistd.h>

int main() {
  struct timeval tv;
  return syscall(SYS_gettimeofday, &tv, 0) == 0 ? 0 : 1;
}
