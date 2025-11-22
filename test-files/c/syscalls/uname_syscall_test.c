#include <sys/syscall.h>
#include <sys/utsname.h>
#include <unistd.h>

int main() {
  struct utsname u;
  return syscall(SYS_uname, &u) == 0 ? 0 : 1;
}
