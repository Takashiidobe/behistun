#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
  pid_t pid = syscall(SYS_fork);
  if (pid < 0) {
    return 1;
  }

  if (pid == 0) {
    syscall(SYS_exit, 0);
    return 1; // Unreachable if syscall worked.
  }

  int status = 0;
  if (syscall(SYS_wait4, pid, &status, 0, 0) < 0) {
    return 1;
  }

  return status == 0 ? 0 : 1;
}
