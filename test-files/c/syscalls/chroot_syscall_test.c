#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Expect failure without privileges; success means dispatch happened.
  return syscall(SYS_chroot, "/") < 0 ? 0 : 1;
}
