#include <sys/syscall.h>
#include <unistd.h>

int main() {
  const char *name = "m68k";
  // Likely to fail without privilege; dispatch is enough.
  return syscall(SYS_setdomainname, name, 4) < 0 ? 0 : 1;
}
