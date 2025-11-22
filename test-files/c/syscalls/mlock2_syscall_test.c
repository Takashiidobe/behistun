#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  char buf[4096];
  syscall(SYS_mlock2, buf, sizeof(buf), 0);
  return 0;
}
