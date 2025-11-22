#include <sys/syscall.h>
#include <sys/sysinfo.h>
#include <unistd.h>

int main() {
  struct sysinfo info;
  return syscall(SYS_sysinfo, &info) == 0 ? 0 : 1;
}
