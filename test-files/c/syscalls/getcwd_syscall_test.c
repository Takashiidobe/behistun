#include <sys/syscall.h>
#include <unistd.h>

int main() {
  char buf[256];
  return syscall(SYS_getcwd, buf, sizeof(buf)) ? 0 : 1;
}
