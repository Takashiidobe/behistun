#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  char buf[4096];
  syscall(SYS_mlock, buf, sizeof(buf));
  long res = syscall(SYS_munlock, buf, sizeof(buf));
  return res == 0 || res < 0 ? 0 : 1;
}
