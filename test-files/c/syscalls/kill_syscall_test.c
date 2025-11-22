#include <sys/syscall.h>
#include <unistd.h>

int main() {
  pid_t pid = getpid();
  return syscall(SYS_kill, pid, 0) == 0 ? 0 : 1;
}
