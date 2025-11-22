#include <sys/syscall.h>
#include <unistd.h>

int main() {
  // Likely to fail without privileges, but exercises dispatch.
  const char *name = "m68k-host";
  return syscall(SYS_sethostname, name, 8) < 0 ? 0 : 1;
}
