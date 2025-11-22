#include <sys/syscall.h>
#include <unistd.h>

int main() {
  long sc = syscall(SYS_getpagesize);
  long api = getpagesize();
  return (sc > 0 && sc == api) ? 0 : 1;
}
