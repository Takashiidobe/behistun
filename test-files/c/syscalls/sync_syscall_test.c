#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_sync);
  return 0;
}
