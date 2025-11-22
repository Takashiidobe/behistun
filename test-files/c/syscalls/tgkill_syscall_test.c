#include <sys/syscall.h>
#include <unistd.h>

int main() {
  pid_t tid = syscall(SYS_gettid);
  return syscall(SYS_tgkill, getpid(), tid, 0) == 0 ? 0 : 1;
}
