#include <sys/statfs.h>
#include <sys/syscall.h>
#include <unistd.h>

int main() {
  struct statfs st;
  return syscall(SYS_statfs, ".", &st) == 0 ? 0 : 1;
}
