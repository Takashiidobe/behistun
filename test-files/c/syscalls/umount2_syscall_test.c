#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Expected to fail without privileges; we just want dispatch.
  return syscall(SYS_umount2, "/", 0) < 0 ? 0 : 1;
}
