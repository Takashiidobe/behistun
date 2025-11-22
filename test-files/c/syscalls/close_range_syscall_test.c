#include <sys/syscall.h>
#include <unistd.h>

int main() {
  syscall(SYS_close_range, 0, ~0U, 0);
  return 0;
}
