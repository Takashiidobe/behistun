#include <sys/syscall.h>
#include <unistd.h>

int main() {
  pid_t pid = syscall(SYS_vfork);
  if (pid < 0) {
    return 0; // Treat failure as dispatched.
  }
  if (pid == 0) {
    syscall(SYS_exit, 0);
    return 1;
  }
  return 0;
}
