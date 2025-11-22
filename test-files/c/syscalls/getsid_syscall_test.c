#include <sys/syscall.h>
#include <unistd.h>

int main() {
  pid_t sid = syscall(SYS_getsid, 0);
  return sid > 0 ? 0 : 1;
}
