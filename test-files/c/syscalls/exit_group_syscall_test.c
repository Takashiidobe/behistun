#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

int main() {
  pid_t pid = syscall(SYS_fork);
  if (pid < 0) {
    return 1;
  }
  if (pid == 0) {
    syscall(SYS_exit_group, 0);
    return 1;
  }
  int status = 0;
  waitpid(pid, &status, 0);
  return status == 0 ? 0 : 1;
}
