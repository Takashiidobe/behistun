#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Expect failure; just dispatch.
  long res = syscall(SYS_init_module, "mod", 0, "");
  (void)res;
  return 0;
}
