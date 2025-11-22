#include <sys/syscall.h>
#include <unistd.h>

int main() {
  pid_t pg = syscall(SYS_getpgid, 0);
  return pg > 0 ? 0 : 1;
}
