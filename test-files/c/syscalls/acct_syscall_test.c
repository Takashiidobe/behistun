#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Expect failure (permission/feature); success means syscall dispatched.
  return syscall(SYS_acct, "nonexistent_acct_file") < 0 ? 0 : 1;
}
