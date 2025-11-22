#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  mode_t old = syscall(SYS_umask, 0022);
  // Restore a common default; ignore errors.
  syscall(SYS_umask, old);
  return old >= 0 ? 0 : 1;
}
